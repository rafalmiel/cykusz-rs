use core::marker::PhantomData;
use x86;

pub trait InOut {
    unsafe fn port_in(port: u16) -> Self;
    unsafe fn port_out(port: u16, value: Self);
}

impl InOut for u8 {
    unsafe fn port_in(port: u16) -> u8 {
        x86::shared::io::inb(port)
    }
    unsafe fn port_out(port: u16, value: u8) {
        x86::shared::io::outb(port, value);
    }
}

impl InOut for u16 {
    unsafe fn port_in(port: u16) -> u16 {
        x86::shared::io::inw(port)
    }
    unsafe fn port_out(port: u16, value: u16) {
        x86::shared::io::outw(port, value);
    }
}

impl InOut for u32 {
    unsafe fn port_in(port: u16) -> u32 {
        x86::shared::io::inl(port)
    }
    unsafe fn port_out(port: u16, value: u32) {
        x86::shared::io::outl(port, value);
    }
}

pub struct Port<T: InOut> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> Port<T> {
    pub const unsafe fn new(port: u16) -> Port<T> {
        Port {
            port: port,
            phantom: PhantomData,
        }
    }

    pub fn read(&mut self) -> T {
        unsafe { T::port_in(self.port) }
    }

    pub fn write(&mut self, value: T) {
        unsafe { T::port_out(self.port, value) }
    }
}

pub struct UnsafePort<T: InOut> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> UnsafePort<T> {
    pub const unsafe fn new(port: u16) -> UnsafePort<T> {
        UnsafePort {
            port: port,
            phantom: PhantomData,
        }
    }

    pub unsafe fn read(&mut self) -> T {
        T::port_in(self.port)
    }

    pub unsafe fn write(&mut self, value: T) {
        T::port_out(self.port, value)
    }
}