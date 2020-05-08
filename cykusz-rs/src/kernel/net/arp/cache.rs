use alloc::collections::btree_map::BTreeMap;

use spin::Once;

use crate::kernel::net::ip::Ip;
use crate::kernel::sync::RwSpin;

struct ArpCache {
    cache: BTreeMap<Ip, [u8; 6]>,
}

impl ArpCache {
    fn insert(&mut self, ip: Ip, mac: &[u8; 6]) {
        self.cache.insert(ip, *mac);
    }

    fn get(&self, ip: Ip) -> Option<[u8; 6]> {
        if let Some(v) = self.cache.get(&ip) {
            return Some(*v);
        } else {
            return None;
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
