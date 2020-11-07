use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;

use spin::Once;

use crate::kernel::net::eth::Eth;
use crate::kernel::net::ip::Ip4;
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
    packets: Vec<Packet<Eth>>,
}

struct ArpCache {
    cache: BTreeMap<Ip4, Entry>,
}

impl ArpCache {
    fn insert(&mut self, ip: Ip4, mac: &[u8; 6]) {
        if let Some(v) = self.cache.get_mut(&ip) {
            if v.status == EntryStatus::Pending {
                v.mac = *mac;
                v.status = EntryStatus::Allocated;
            }

            for p in &v.packets {
                println!("[ ARP ] Send cached packet");
                crate::kernel::net::eth::send_packet_to_mac(*p, &v.mac);
            }

            v.packets.clear();
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

    fn get(&self, ip: Ip4) -> Option<[u8; 6]> {
        if let Some(v) = self.cache.get(&ip) {
            if v.status == EntryStatus::Allocated {
                return Some(v.mac);
            }
        }

        return None;
    }

    fn request(&mut self, ip: Ip4, packet: Packet<Eth>) {
        if let Some(v) = self.cache.get_mut(&ip) {
            if v.status == EntryStatus::Pending {
                println!("[ ARP ] Enqueuing packet");
                v.packets.push(packet)
            } else {
                //panic!("How did we get here");
            }
        } else {
            let mut vec = Vec::new();
            vec.push(packet);
            println!("[ ARP ] Enqueuing packet");

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

pub fn insert(ip: Ip4, mac: &[u8; 6]) {
    //println!("[ ARP ] Cache {:?} -> {:?}", ip, mac);

    CACHE.get().as_ref().unwrap().write().insert(ip, mac);
}

pub fn get(ip: Ip4) -> Option<[u8; 6]> {
    CACHE.get().as_ref().unwrap().read().get(ip)
}

pub fn request_ip(ip: Ip4, packet: Packet<Eth>) {
    let mut cache = CACHE.get().as_ref().unwrap().write();

    cache.request(ip, packet);
}

pub fn init() {
    CACHE.call_once(|| {
        let mut c = ArpCache {
            cache: BTreeMap::new(),
        };

        c.insert(
            Ip4::limited_broadcast(),
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );

        RwSpin::new(c)
    });
}
