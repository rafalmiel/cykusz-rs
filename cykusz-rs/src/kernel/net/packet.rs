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
    fn base_len(&self) -> usize;
    fn addr(&self) -> VirtAddr;
    fn len(&self) -> usize;
    fn auto_remove(&self) -> bool;
    fn set_auto_remove(&mut self, v: bool);
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
    pub base_len: usize,
    pub addr: VirtAddr,
    pub len: usize,
    pub auto_remove: bool,
    _p: PhantomData<T>,
}

impl<T: PacketKind> PacketBaseTrait for Packet<T> {
    fn base_addr(&self) -> VirtAddr {
        self.base_addr
    }

    fn base_len(&self) -> usize {
        self.base_len
    }

    fn addr(&self) -> VirtAddr {
        self.addr
    }

    fn len(&self) -> usize {
        self.len
    }

    fn auto_remove(&self) -> bool {
        self.auto_remove
    }

    fn set_auto_remove(&mut self, v: bool) {
        self.auto_remove = v;
    }
}

impl<T: PacketKind> Packet<T> {
    pub fn new(addr: VirtAddr, len: usize) -> Packet<T> {
        Packet::<T> {
            base_addr: addr,
            base_len: len,
            addr,
            len,
            auto_remove: true,
            _p: PhantomData::default(),
        }
    }

    fn new_base(
        base_addr: VirtAddr,
        base_len: usize,
        addr: VirtAddr,
        len: usize,
        auto_remove: bool,
    ) -> Packet<T> {
        Packet::<T> {
            base_addr,
            base_len,
            addr,
            len,
            auto_remove,
            _p: PhantomData::default(),
        }
    }

    pub fn eth(&self) -> Packet<Eth> {
        Packet::<Eth>::new(self.base_addr(), self.base_len())
    }

    pub fn deallocate(self) {
        crate::kernel::net::eth::dealloc_packet(self.eth());
    }
}

pub trait PacketUpHierarchy<B: PacketKind>: PacketTrait {
    fn upgrade(&self) -> Packet<B> {
        let hs = self.header_size();
        Packet::<B>::new_base(
            self.base_addr(),
            self.base_len(),
            self.addr() + hs,
            self.len() - hs,
            self.auto_remove(),
        )
    }
}

pub trait PacketDownHierarchy<B: PacketKind>: PacketBaseTrait {
    fn downgrade(&self) -> Packet<B>;

    fn downgrade_by(&self, amount: usize) -> Packet<B> {
        Packet::<B>::new_base(
            self.base_addr(),
            self.base_len(),
            self.addr() - amount,
            self.len() + amount,
            self.auto_remove(),
        )
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
