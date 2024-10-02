use std::collections::{HashMap, LinkedList};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::ExitCode;
use syscall_defs::poll::{PollEventFlags, PollFd};
use syscall_defs::{MMapFlags, MMapProt};

#[repr(transparent)]
struct SoundChunk([i16; 1024]);

#[repr(transparent)]
struct MixChunk([i32; 1024]);

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
}

impl Client {
    fn new(input: UnixStream) -> Client {
        Client {
            input,
            chunks: LinkedList::new(),
            disconnected: false,
        }
    }

    fn fetch(&mut self) -> usize {
        let mut data = SoundChunk::new();

        if let Ok(n) = self.input.read(data.as_bytes_mut()) {
            if n == 0 {
                return 0;
            }
            self.chunks.push_back(data);

            n
        } else {
            0
        }
    }

    fn pop(&mut self) -> Option<SoundChunk> {
        self.chunks.pop_front()
    }
}

struct Output<'a> {
    listener: UnixListener,
    clients: HashMap<i32, Client>,
    pos: &'a [u64; 512],
    sound: &'a mut [SoundChunk; 32],
    write_pos: u64,
    poll_fds: Vec<PollFd>,
}

impl<'a> Output<'a> {
    fn new() -> Result<Output<'a>, ExitCode> {
        let hda_dev = std::fs::File::open("/dev/hda").map_err(|_e| ExitCode::from(1))?;

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
            clients: HashMap::new(),
            pos: unsafe { &*(hda_map as *const [u64; 512]) },
            write_pos: 0,
            sound: unsafe { &mut *((hda_map + 4096) as *mut [SoundChunk; 32]) },
            poll_fds: Vec::new(),
        };

        output.reinit_fds();

        Ok(output)
    }

    fn listener(&self) -> &UnixListener {
        &self.listener
    }

    fn add_client(&mut self, client: Client) {
        self.clients.insert(client.input.as_raw_fd(), client);
    }

    fn fetch(&mut self, client_id: i32) -> usize {
        if let Some(c) = self.clients.get_mut(&client_id) {
            c.fetch()
        } else {
            0
        }
    }

    fn remove(&mut self, client_id: i32) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.disconnected = true;
        }
    }

    fn reinit_fds(&mut self) {
        self.poll_fds.clear();
        self.poll_fds
            .push(PollFd::new(self.listener.as_raw_fd(), PollEventFlags::READ));
        for c in self.clients.values() {
            if !c.disconnected {
                self.poll_fds
                    .push(PollFd::new(c.input.as_raw_fd(), PollEventFlags::READ));
            }
        }
    }

    fn poll_fds(&mut self) -> Vec<PollFd> {
        if self.poll_fds.len() != self.clients.len() + 1 {
            self.reinit_fds();
        }
        self.poll_fds.clone()
    }

    fn process(&mut self) {
        let mut to_delete = Vec::new();
        if (self.pos[4] / 2048 + 3) % 32 == self.write_pos {
            //println!("got pos: {}", self.pos[4] / 2048);
            let mut chunk = MixChunk::new();

            for c in self.clients.values_mut() {
                if let Some(s) = c.pop() {
                    chunk.mix(&s);
                } else if c.disconnected {
                    to_delete.push(c.input.as_raw_fd());
                }
            }

            self.sound[self.write_pos as usize] = chunk.to_sound_chunk();

            self.write_pos = (self.write_pos + 1) % 32;
        }

        for c in &to_delete {
            self.clients.remove(c);
        }
    }
}

fn sound_daemon() -> Result<(), ExitCode> {
    let _ = std::fs::remove_file("/sound-daemon.pid");
    let mut output = Output::new()?;

    loop {
        let mut polls = output.poll_fds();
        if let Ok(res) = syscall_user::poll(polls.as_mut_slice(), 0) {
            if res > 0 {
                for (id, ev) in polls.iter().enumerate() {
                    if id == 0 && ev.revents.contains(PollEventFlags::READ) {
                        match output.listener().accept() {
                            Ok((s, _addr)) => {
                                output.add_client(Client::new(s));
                            }
                            Err(_e) => {
                                //println!("server: accept err: {:?}", e);
                            }
                        }
                    } else if id > 0 && ev.revents.contains(PollEventFlags::READ) {
                        if output.fetch(ev.fd) == 0 && ev.revents.contains(PollEventFlags::HUP) {
                            output.remove(ev.fd);
                        }
                    }
                }
            }
        };

        output.process();
    }
}

fn main() -> Result<(), ExitCode> {
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
