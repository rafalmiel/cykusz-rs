use core::marker::PhantomData;

use crate::kernel::mm::VirtAddr;
use crate::kernel::net::eth::Eth;

pub trait PacketKind {}

pub trait ConstPacketKind: PacketKind {
    const HSIZE: usize;
}

impl<T: ConstPacketKind> PacketKind for T {}

impl<U, D> PacketDownHierarchy<D> for Packet<U>
where
    U: PacketKind,
    D: ConstPacketKind,
    Packet<D>: PacketUpHierarchy<U>,
{
}

pub trait PacketBaseTrait {
    fn addr(&self) -> VirtAddr;
    fn len(&self) -> usize;
}

pub trait PacketTrait: PacketBaseTrait {
    fn header_size(&self) -> usize;

    fn data(&self) -> &[u8] {
        let hsize = self.header_size();
        unsafe {
            core::slice::from_raw_parts((self.addr() + hsize).0 as *const u8, self.len() - hsize)
        }
    }

    fn data_mut(&mut self) -> &mut [u8] {
        let hsize = self.header_size();
        unsafe {
            core::slice::from_raw_parts_mut((self.addr() + hsize).0 as *mut u8, self.len() - hsize)
        }
    }
}

impl<T: ConstPacketKind> PacketTrait for Packet<T> {
    fn header_size(&self) -> usize {
        T::HSIZE
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Packet<T: PacketKind> {
    pub addr: VirtAddr,
    len: usize,
    _p: PhantomData<T>,
}

impl<T: PacketKind> PacketBaseTrait for Packet<T> {
    fn addr(&self) -> VirtAddr {
        self.addr
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl<T: PacketKind> Packet<T> {
    pub fn new(addr: VirtAddr, len: usize) -> Packet<T> {
        Packet::<T> {
            addr,
            len,
            _p: PhantomData::default(),
        }
    }
}

pub trait PacketUpHierarchy<B: PacketKind>: PacketTrait {
    fn upgrade(&self) -> Packet<B> {
        let hs = self.header_size();
        Packet::<B>::new(self.addr() + hs, self.len() - hs)
    }
}

pub trait PacketDownHierarchy<B: ConstPacketKind>: PacketBaseTrait {
    fn downgrade(&self) -> Packet<B> {
        let hs = B::HSIZE;
        Packet::<B>::new(self.addr() - hs, self.len() + hs)
    }
}

pub trait PacketHeader<H>: PacketBaseTrait {
    fn header(&self) -> &H {
        unsafe { self.addr().read_ref::<H>() }
    }

    fn header_mut(&mut self) -> &mut H {
        unsafe { self.addr().read_mut::<H>() }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RecvPacket {
    pub packet: Packet<Eth>,
    pub id: usize,
}
