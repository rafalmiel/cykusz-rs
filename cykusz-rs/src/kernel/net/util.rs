#![allow(dead_code)]

pub mod checksum {
    use syscall_defs::net::NetU16;

    use crate::kernel::net::ip::{Ip4, IpHeader, IpType};

    #[repr(packed)]
    pub struct PseudoHeader {
        src_ip: Ip4,
        dst_ip: Ip4,
        zero: u8,
        prot: IpType,
        len: NetU16,
    }

    impl PseudoHeader {
        pub fn new(ip_hdr: &IpHeader) -> PseudoHeader {
            PseudoHeader {
                src_ip: ip_hdr.src_ip,
                dst_ip: ip_hdr.dest_ip,
                zero: 0,
                prot: ip_hdr.protocol,
                len: NetU16::new(ip_hdr.len.value() - core::mem::size_of::<IpHeader>() as u16),
            }
        }
    }

    fn calc_checksum(data: &[u8]) -> u32 {
        let mut sum = 0;

        let ptr = data.as_ptr() as *const NetU16;

        for i in 0..(data.len() / 2) {
            sum += unsafe { (*ptr.offset(i as isize)).value() as u32 }
        }

        if data.len() % 2 == 1 {
            sum += ((*data.last().unwrap()) as u32) << 8;
        }

        sum
    }

    pub fn make(mut sum: u32) -> NetU16 {
        let mut carry = sum >> 16;
        while carry > 0 {
            sum &= 0x0000_ffff;
            sum += carry;
            carry = sum >> 16;
        }
        NetU16::new(!(sum as u16))
    }

    pub fn make_combine(a: &[u32]) -> NetU16 {
        make(a.iter().sum())
    }

    pub fn calc_ref<T: Sized>(r: &T) -> u32 {
        calc_checksum(ref_to_bytes(r))
    }

    pub fn calc_ref_len<T: ?Sized>(r: &T, len: usize) -> u32 {
        calc_checksum(ref_to_bytes_len(r, len))
    }

    fn ref_to_bytes<T: Sized>(r: &T) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(r as *const _ as *const u8, core::mem::size_of::<T>())
        }
    }

    fn ref_to_bytes_len<T: ?Sized>(r: &T, len: usize) -> &[u8] {
        unsafe { core::slice::from_raw_parts(r as *const _ as *const u8, len) }
    }
}
