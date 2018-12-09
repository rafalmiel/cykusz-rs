use kernel::mm::*;

pub fn get_user_program() -> MappedAddr {
    ::arch::user::get_user_program()
}

pub fn get_user_program_size() -> u64 {
    ::arch::user::get_user_program_size()
}