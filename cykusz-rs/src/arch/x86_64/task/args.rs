use alloc::string::String;
use alloc::vec::Vec;

use crate::arch::utils::StackHelper;

pub struct Args {
    args: Vec<String>,
}

impl Args {
    pub fn from_ref(arg: &[&str]) -> Args {
        let mut vec = Vec::<String>::new();

        for a in arg.iter() {
            vec.push(String::from(*a));
        }

        Args { args: vec }
    }

    pub fn write_strings(&self, helper: &mut StackHelper) -> Vec<u64> {
        let mut pos = Vec::<u64>::with_capacity(self.args.len());

        for (_i, e) in self.args.iter().enumerate() {
            unsafe {
                helper.write(0u8);
                helper.write_bytes(e.as_bytes());
            }

            pos.push(helper.current());
        }

        pos
    }
}
