use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem::MaybeUninit;

pub type ExeArgs = Vec<Box<[u8]>>;

pub fn into_syscall_slice(args: &[&str]) -> Vec<[usize; 2]> {
    let mut vec = Vec::<[usize; 2]>::with_capacity(args.len());

    for s in args.iter() {
        vec.push([s.as_ptr() as usize, s.len()]);
    }

    vec
}

pub fn from_syscall_slice(args: usize, len: usize) -> ExeArgs {
    let slice = unsafe { core::slice::from_raw_parts(args as *const [usize; 2], len) };

    let mut vec = ExeArgs::new();

    for s in slice.iter() {
        let vals = {
            let mut b = Box::new_uninit_slice(s[1]);
            unsafe {
                b.as_mut_ptr()
                    .copy_from(s[0] as *const MaybeUninit<u8>, s[1]);
                b.assume_init()
            }
        };

        vec.push(vals);
    }

    vec
}
