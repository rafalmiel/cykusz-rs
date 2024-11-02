use bbqueue::{BBBuffer, Consumer, Producer};
use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::{size_of, MaybeUninit};
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Once;

unsafe impl<'a, T: Sized + Debug, const N: usize> Sync for SPSCQueue<'a, T, N> where
    [(); N * size_of::<T>()]:
{
}
unsafe impl<'a, T: Sized + Debug, const N: usize> Send for SPSCQueue<'a, T, N> where
    [(); N * size_of::<T>()]:
{
}

pub struct SPSCQueue<'a, T: Sized + Debug, const N: usize>
where
    [(); N * size_of::<T>()]:,
{
    p_data: PhantomData<T>,
    queue: BBBuffer<{ N * size_of::<T>() }>,

    prod: Once<core::cell::RefCell<bbqueue::Producer<'a, { N * size_of::<T>() }>>>,
    cons: Once<core::cell::RefCell<bbqueue::Consumer<'a, { N * size_of::<T>() }>>>,

    data_count: AtomicUsize,
}

impl<'a, T: Sized + Debug + 'a, const N: usize> SPSCQueue<'a, T, N>
where
    [(); N * size_of::<T>()]:,
{
    pub fn new() -> SPSCQueue<'a, T, N> {
        let bb = BBBuffer::<{ N * size_of::<T>() }>::new();

        let spsc = SPSCQueue {
            p_data: PhantomData::default(),
            queue: bb,
            prod: Once::new(),
            cons: Once::new(),
            data_count: AtomicUsize::new(0),
        };

        spsc
    }

    pub(crate) fn init<'b: 'a>(&'b self) {
        let (prod, cons) = self.queue.try_split().unwrap();

        self.prod.call_once(move || core::cell::RefCell::new(prod));
        self.cons.call_once(move || core::cell::RefCell::new(cons));
    }

    fn prod(&self) -> core::cell::RefMut<Producer<'a, { N * size_of::<T>() }>> {
        unsafe { self.prod.get_unchecked().borrow_mut() }
    }

    fn cons(&self) -> core::cell::RefMut<Consumer<'a, { N * size_of::<T>() }>> {
        unsafe { self.cons.get_unchecked().borrow_mut() }
    }

    pub fn has_data(&self) -> bool {
        self.data_count.load(Ordering::Relaxed) > 0
    }

    pub fn try_write_one(&self, data: &T) -> Option<usize> {
        let mut prod = self.prod();

        let mut grant = prod.grant_exact(size_of::<T>()).ok()?;

        grant.buf().copy_from_slice(unsafe {
            core::slice::from_raw_parts(data as *const _ as *const u8, size_of::<T>())
        });

        grant.commit(size_of::<T>());

        self.data_count.fetch_add(1, Ordering::Relaxed);

        Some(size_of::<T>())
    }

    pub fn try_read_one(&self) -> Option<T> {
        let mut cons = self.cons();

        let grant = cons.read().ok()?;

        if grant.len() < size_of::<T>() {
            return None;
        }

        let mut item = MaybeUninit::<T>::uninit();

        unsafe {
            core::slice::from_raw_parts_mut(item.as_mut_ptr() as *mut u8, size_of::<T>())
                .copy_from_slice(&grant.buf()[..size_of::<T>()]);
        }

        let res = Some(unsafe { item.assume_init() });

        grant.release(size_of::<T>());

        self.data_count.fetch_sub(1, Ordering::Relaxed);

        res
    }
}
