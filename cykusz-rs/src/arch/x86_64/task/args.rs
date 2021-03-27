use alloc::vec::Vec;

use syscall_defs::exec::ExeArgs;

use crate::arch::utils::StackHelper;

pub struct Args {
    args: ExeArgs,
}

impl Args {
    pub fn new(exe: ExeArgs) -> Args {
        Args { args: exe }
    }

    pub fn write_strings(&self, helper: &mut StackHelper) -> Vec<u64> {
        let mut pos = Vec::<u64>::with_capacity(self.args.len());

        for (_i, e) in self.args.iter().enumerate() {
            unsafe {
                helper.write(0u8);
                helper.write_bytes(e);
            }

            pos.push(helper.current());
        }

        pos
    }
}
