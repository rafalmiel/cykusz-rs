use crate::time::Timeval;

pub mod keys;
pub mod buttons;

#[repr(u16)]
#[derive(Debug)]
pub enum EventType {
    Key = 1,
    Rel = 2,
}

#[repr(C)]
#[derive(Debug)]
pub struct Event {
    pub timeval: Timeval,
    pub typ: EventType,
    pub code: u16,
    pub val: i32,
}
