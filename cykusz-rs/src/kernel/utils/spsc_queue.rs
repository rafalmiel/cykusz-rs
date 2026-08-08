use bbqueue::nicknames::Churrasco;

use core::fmt::Debug;
use core::mem::{MaybeUninit, size_of};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Inline Storage, Atomics, Polling, Borrowed
type Queue<T: Sized + Debug, const N: usize> = Churrasco<{ N * size_of::<T>() }>;

pub struct SPSCQueue<T: Sized + Debug, const N: usize>
where
    [(); N * size_of::<T>()]:,
{
    queue: Queue<T, N>,

    data_count: AtomicUsize,
}

impl<T: Sized + Debug, const N: usize> SPSCQueue<T, N>
where
    [(); N * size_of::<T>()]:,
{
    pub fn new() -> SPSCQueue<T, N> {
        let bb = Queue::<T, N>::new();

        let spsc = SPSCQueue {
            queue: bb,
            data_count: AtomicUsize::new(0),
        };

        spsc
    }

    pub fn has_data(&self) -> bool {
        self.data_count.load(Ordering::Relaxed) > 0
    }

    pub fn try_write_one(&self, data: &T) -> Option<usize> {
        let prod = self.queue.framed_producer();

        let mut grant = prod.grant(size_of::<T>() as u16).ok()?;

        grant.copy_from_slice(unsafe {
            core::slice::from_raw_parts(data as *const _ as *const u8, size_of::<T>())
        });

        grant.commit(size_of::<T>() as u16);

        self.data_count.fetch_add(1, Ordering::Relaxed);

        Some(size_of::<T>())
    }

    pub fn try_read_one(&self) -> Option<T> {
        let cons = self.queue.framed_consumer();

        let grant = cons.read().ok()?;

        if grant.len() < size_of::<T>() {
            return None;
        }

        let mut item = MaybeUninit::<T>::uninit();

        unsafe {
            core::slice::from_raw_parts_mut(item.as_mut_ptr() as *mut u8, size_of::<T>())
                .copy_from_slice(&grant[..size_of::<T>()]);
        }

        let res = Some(unsafe { item.assume_init() });

        self.data_count.fetch_sub(1, Ordering::Relaxed);

        grant.release();

        res
    }
}
