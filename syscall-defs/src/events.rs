#[repr(u16)]
#[derive(Debug)]
pub enum EventType {
    Key = 1,
}

#[repr(C)]
#[derive(Debug)]
pub struct Event {
    pub typ: EventType,
    pub code: u16,
    pub val: i32,
}
