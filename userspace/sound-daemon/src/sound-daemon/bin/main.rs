use std::collections::{HashMap, LinkedList};
use std::fs::File;
use std::io::Read;
use std::ops::Add;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::ExitCode;
use syscall_defs::poll::{PollEventFlags, PollFd};
use syscall_defs::{MMapFlags, MMapProt};

const CHUNK_SIZE: usize = 2048;
#[repr(transparent)]
struct SoundChunk([i16; CHUNK_SIZE / 2]);

#[repr(transparent)]
struct MixChunk([i32; CHUNK_SIZE / 2]);

const CHUNK_COUNT: u64 = 32;
const WRITE_HEADROOM: u64 = 3;

const MAX_CHUNKS_IN_BUF: usize = 16;

#[derive(Copy, Clone, Eq, PartialEq)]
enum FetchResult {
    Full,
    More,
    None,
}

type HdaWritePos = WritePos<CHUNK_COUNT>;

#[derive(Eq, PartialEq, Copy, Clone)]
struct WritePos<const MAX: u64>(pub u64);

impl<const MAX: u64> WritePos<MAX> {
    fn from_pos(pos: u64) -> Self {
        WritePos(pos / 2048)
    }

    fn distance_to(&self, other: Self) -> usize {
        if self.0 <= other.0 {
            // ...S....O...MAX
            other.0 as usize - self.0 as usize
        } else {
            // ...O....S...MAX
            MAX as usize - self.0 as usize + other.0 as usize
        }
    }
}

impl<const MAX: u64> From<WritePos<MAX>> for usize {
    fn from(value: WritePos<MAX>) -> Self {
        value.0 as usize
    }
}

impl<const MAX: u64> Add for WritePos<MAX> {
    type Output = WritePos<MAX>;

    fn add(self, rhs: Self) -> Self::Output {
        WritePos::<MAX>((self.0 + rhs.0) % MAX)
    }
}

impl<const MAX: u64> Add<u64> for WritePos<MAX> {
    type Output = WritePos<MAX>;

    fn add(self, rhs: u64) -> Self::Output {
        WritePos::<MAX>((self.0 + rhs) % MAX)
    }
}

impl MixChunk {
    fn new() -> MixChunk {
        MixChunk([0i32; 1024])
    }

    fn mix(&mut self, chunk: &SoundChunk) {
        self.0.iter_mut().zip(chunk.0.iter()).for_each(|(m, s)| {
            *m += *s as i32;
        })
    }

    fn to_sound_chunk(&self) -> SoundChunk {
        let mut s = SoundChunk::new();
        s.0.iter_mut().zip(self.0.iter()).for_each(|(s, m)| {
            *s = (*m).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        });
        s
    }
}

impl SoundChunk {
    fn new() -> SoundChunk {
        SoundChunk([0i16; 1024])
    }

    fn as_bytes_mut(&mut self) -> &mut [u8; 2048] {
        unsafe { std::mem::transmute::<&mut [i16; 1024], &mut [u8; 2048]>(&mut self.0) }
    }
}

struct Client {
    input: UnixStream,
    chunks: LinkedList<SoundChunk>,
    disconnected: bool,
    full: bool,
}

impl Client {
    fn new(input: UnixStream) -> Client {
        Client {
            input,
            chunks: LinkedList::new(),
            disconnected: false,
            full: false,
        }
    }

    fn fetch(&mut self) -> FetchResult {
        let mut data = SoundChunk::new();

        if let Ok(n) = self.input.read(data.as_bytes_mut()) {
            if n == 0 {
                return FetchResult::None;
            } else if n == CHUNK_SIZE {
                self.chunks.push_back(data);
            }

            if self.chunks.len() == MAX_CHUNKS_IN_BUF {
                FetchResult::Full
            } else {
                FetchResult::More
            }
        } else {
            FetchResult::None
        }
    }

    fn pop(&mut self) -> Option<SoundChunk> {
        self.chunks.pop_front()
    }
}

struct Output<'a> {
    listener: UnixListener,
    hda_dev: File,
    clients: HashMap<i32, Client>,
    pos: &'a [u64; 512],
    sound: &'a mut [SoundChunk; 32],
    last_write_pos: HdaWritePos,
    poll_fds: Vec<PollFd>,
}

impl<'a> Output<'a> {
    fn new() -> Result<Output<'a>, ExitCode> {
        let hda_dev = File::open("/dev/hda").map_err(|_e| ExitCode::from(1))?;

        let hda_map = syscall_user::mmap(
            None,
            2048 * 32 + 4096,
            MMapProt::PROT_READ | MMapProt::PROT_WRITE,
            MMapFlags::MAP_SHARED,
            Some(hda_dev.as_raw_fd() as usize),
            0,
        )
        .map_err(|_e| ExitCode::from(2))?;

        let listener = UnixListener::bind("/sound-daemon.pid").expect("Sound Daemon running?");

        let mut output = Output::<'a> {
            listener,
            hda_dev,
            clients: HashMap::new(),
            pos: unsafe { &*(hda_map as *const [u64; 512]) },
            last_write_pos: HdaWritePos::from_pos(0),
            sound: unsafe { &mut *((hda_map + 4096) as *mut [SoundChunk; 32]) },
            poll_fds: Vec::new(),
        };

        // 3 chunks headroom
        output.last_write_pos = output.hda_write_pos() + WRITE_HEADROOM;

        output.reinit_fds();

        Ok(output)
    }

    fn listener(&self) -> &UnixListener {
        &self.listener
    }

    fn add_client(&mut self, client: Client) {
        self.clients.insert(client.input.as_raw_fd(), client);
        self.reinit_fds();
    }

    fn fetch(&mut self, client_id: i32) -> FetchResult {
        if let Some(c) = self.clients.get_mut(&client_id) {
            c.fetch()
        } else {
            FetchResult::None
        }
    }

    fn remove(&mut self, client_id: i32) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.disconnected = true;

            self.reinit_fds();
        }
    }

    fn mark_full(&mut self, client_id: i32) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.full = true;

            self.reinit_fds();
        }
    }

    fn reinit_fds(&mut self) {
        self.poll_fds.clear();
        self.poll_fds
            .push(PollFd::new(self.listener.as_raw_fd(), PollEventFlags::READ));
        self.poll_fds
            .push(PollFd::new(self.hda_dev.as_raw_fd(), PollEventFlags::WRITE));
        for c in self.clients.values() {
            if !c.disconnected && !c.full {
                self.poll_fds
                    .push(PollFd::new(c.input.as_raw_fd(), PollEventFlags::READ));
            }
        }
    }

    fn hda_write_pos(&self) -> HdaWritePos {
        HdaWritePos::from_pos(self.pos[4])
    }

    fn poll_fds(&mut self) -> Vec<PollFd> {
        self.poll_fds.clone()
    }

    fn process(&mut self) {
        let mut to_delete = Vec::new();
        let current_hda_pos = self.hda_write_pos();
        let expected_hda_pos = self.hda_write_pos() + 3;
        let mut reinit = false;
        while expected_hda_pos != self.last_write_pos {
            //println!("got pos: {}", self.pos[4] / 2048);
            let do_mix = current_hda_pos.distance_to(self.last_write_pos) < WRITE_HEADROOM as usize;
            let mut chunk = MixChunk::new();

            for c in self.clients.values_mut() {
                if let Some(s) = c.pop() {
                    if do_mix {
                        chunk.mix(&s);
                    }
                    if c.full {
                        c.full = false;
                        reinit = true;
                    }
                } else if c.disconnected {
                    to_delete.push(c.input.as_raw_fd());
                    c.disconnected = false; // to not push into vector twice
                }
            }

            self.sound[self.last_write_pos.0 as usize] = chunk.to_sound_chunk();

            self.last_write_pos = self.last_write_pos + 1;
        }

        self.last_write_pos = expected_hda_pos;

        for c in &to_delete {
            self.clients.remove(c);
        }

        if reinit {
            self.reinit_fds();
        }
    }
}

fn sound_daemon() -> Result<(), ExitCode> {
    let _ = std::fs::remove_file("/sound-daemon.pid");
    let mut output = Output::new()?;

    loop {
        let mut polls = output.poll_fds();
        if let Ok(res) = syscall_user::poll(polls.as_mut_slice(), -1) {
            if res > 0 {
                for (id, ev) in polls.iter().enumerate() {
                    let is_read = ev.revents.contains(PollEventFlags::READ);
                    let is_hup = ev.revents.contains(PollEventFlags::HUP);
                    match id {
                        0 if is_read => {
                            match output.listener().accept() {
                                Ok((s, _addr)) => {
                                    output.add_client(Client::new(s));
                                }
                                Err(_e) => {
                                    //println!("server: accept err: {:?}", e);
                                }
                            }
                        }
                        a if a > 1 && is_read => match output.fetch(ev.fd) {
                            FetchResult::None if is_hup => {
                                output.remove(ev.fd);
                            }
                            FetchResult::Full => {
                                output.mark_full(ev.fd);
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            output.process();
        };
    }
}

fn main() -> Result<(), ExitCode> {
    //return Ok(sound_daemon()?);
    let daemon = syscall_user::fork().expect("fork failed");

    if daemon > 0 {
        return Ok(());
    }

    let _sid = syscall_user::setsid().expect("setsid failed");

    syscall_user::chdir("/").expect("chdir failed");

    syscall_user::close(0).expect("close 0 faield");
    syscall_user::close(1).expect("close 1 faield");
    syscall_user::close(2).expect("close 2 faield");

    Ok(sound_daemon()?)
}
