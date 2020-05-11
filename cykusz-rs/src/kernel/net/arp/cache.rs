use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;

use spin::Once;

use crate::kernel::net::ip::Ip;
use crate::kernel::net::Packet;
use crate::kernel::sync::RwSpin;

#[derive(Eq, PartialEq)]
enum EntryStatus {
    Allocated = 0x1,
    Pending = 0x2,
}

struct Entry {
    mac: [u8; 6],
    status: EntryStatus,
    packets: Vec<Packet>,
}

struct ArpCache {
    cache: BTreeMap<Ip, Entry>,
}

impl ArpCache {
    fn insert(&mut self, ip: Ip, mac: &[u8; 6]) {
        if let Some(v) = self.cache.get_mut(&ip) {
            if v.status == EntryStatus::Pending {
                v.mac = *mac;
                v.status = EntryStatus::Allocated;
            }

            for p in &v.packets {
                crate::kernel::net::eth::send_packet_to_mac(*p, &v.mac);
            }
        } else {
            self.cache.insert(
                ip,
                Entry {
                    mac: *mac,
                    status: EntryStatus::Allocated,
                    packets: Vec::new(),
                },
            );
        }
    }

    fn get(&self, ip: Ip) -> Option<[u8; 6]> {
        if let Some(v) = self.cache.get(&ip) {
            if v.status == EntryStatus::Allocated {
                return Some(v.mac);
            }
        }

        return None;
    }

    fn request(&mut self, ip: Ip, packet: Packet) {
        if let Some(v) = self.cache.get_mut(&ip) {
            if v.status == EntryStatus::Pending {
                v.packets.push(packet)
            } else {
                panic!("How did we get here");
            }
        } else {
            let mut vec = Vec::new();
            vec.push(packet);

            self.cache.insert(
                ip,
                Entry {
                    mac: [0, 0, 0, 0, 0, 0],
                    status: EntryStatus::Pending,
                    packets: vec,
                },
            );
        }
    }
}

static CACHE: Once<RwSpin<ArpCache>> = Once::new();

pub fn insert(ip: Ip, mac: &[u8; 6]) {
    //println!("[ ARP ] Cache {:?} -> {:?}", ip, mac);

    CACHE.r#try().as_ref().unwrap().write().insert(ip, mac);
}

pub fn get(ip: Ip) -> Option<[u8; 6]> {
    CACHE.r#try().as_ref().unwrap().read().get(ip)
}

pub fn request_ip(ip: Ip, packet: Packet) {
    let mut cache = CACHE.r#try().as_ref().unwrap().write();

    cache.request(ip, packet);
}

pub fn init() {
    CACHE.call_once(|| {
        let mut c = ArpCache {
            cache: BTreeMap::new(),
        };

        c.insert(
            Ip::limited_broadcast(),
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );

        RwSpin::new(c)
    });
}
