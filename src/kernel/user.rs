use crate::kernel::mm::*;

pub fn get_user_program() -> MappedAddr {
    crate::arch::user::get_user_program()
}

pub fn get_user_program_size() -> u64 {
    crate::arch::user::get_user_program_size()
}
