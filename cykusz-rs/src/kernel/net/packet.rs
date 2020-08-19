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
    fn downgrade(&self) -> Packet<D> {
        self.downgrade_by(D::HSIZE)
    }
}

pub trait PacketBaseTrait {
    fn base_addr(&self) -> VirtAddr;
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
    pub base_addr: VirtAddr,
    pub addr: VirtAddr,
    pub len: usize,
    _p: PhantomData<T>,
}

impl<T: PacketKind> PacketBaseTrait for Packet<T> {
    fn base_addr(&self) -> VirtAddr {
        self.base_addr
    }

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
            base_addr: addr,
            addr,
            len,
            _p: PhantomData::default(),
        }
    }

    pub fn new_base(base_addr: VirtAddr, addr: VirtAddr, len: usize) -> Packet<T> {
        Packet::<T> {
            base_addr,
            addr,
            len,
            _p: PhantomData::default(),
        }
    }
}

pub trait PacketUpHierarchy<B: PacketKind>: PacketTrait {
    fn upgrade(&self) -> Packet<B> {
        let hs = self.header_size();
        Packet::<B>::new_base(self.base_addr(), self.addr() + hs, self.len() - hs)
    }
}

pub trait PacketDownHierarchy<B: PacketKind>: PacketBaseTrait {
    fn downgrade(&self) -> Packet<B>;

    fn downgrade_by(&self, amount: usize) -> Packet<B> {
        Packet::<B>::new_base(self.base_addr(), self.addr() - amount, self.len() + amount)
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
