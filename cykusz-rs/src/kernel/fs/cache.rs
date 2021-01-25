use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use core::borrow::Borrow;
use core::fmt::Debug;
use core::hash::Hash;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::{AtomicBool, Ordering};

use lru::LruCache;

use crate::kernel::sync::Spin;

pub trait IsCacheKey: Eq + Hash + Ord + Borrow<Self> + Debug {}

impl<T> IsCacheKey for T where T: Eq + Hash + Ord + Borrow<Self> + Debug {}

pub trait Cacheable<K: IsCacheKey>: Clone {
    fn cache_key(&self) -> K;

    fn make_unused(&self, _new_ref: &Weak<CacheItem<K, Self>>) {}
}

pub struct CacheItem<K: IsCacheKey, T: Cacheable<K>> {
    cache: Weak<Cache<K, T>>,
    used: AtomicBool,
    val: T,
}

impl<K: IsCacheKey, T: Cacheable<K>> Clone for CacheItem<K, T> {
    fn clone(&self) -> CacheItem<K, T> {
        CacheItem::<K, T> {
            cache: self.cache.clone(),
            used: AtomicBool::new(self.used.load(Ordering::SeqCst)),
            val: self.val.clone(),
        }
    }
}

impl<K: IsCacheKey, T: Cacheable<K>> Deref for CacheItem<K, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<K: IsCacheKey, T: Cacheable<K>> DerefMut for CacheItem<K, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

impl<K: IsCacheKey, T: Cacheable<K>> Drop for CacheItem<K, T> {
    fn drop(&mut self) {
        //println!(
        //    "drop DirEntry {:?} is used {}",
        //    self.cache_key(),
        //    self.is_used()
        //);
        if let Some(cache) = self.cache.upgrade() {
            if self.is_used() {
                self.mark_unused();

                cache.move_to_unused(self.clone());

                //println!("moved to unused {:?}", self.cache_key());
            } else {
            }
        }
    }
}

impl<K: IsCacheKey, T: Cacheable<K>> CacheItem<K, T> {
    pub fn mark_used(&self) {
        self.used.store(true, Ordering::SeqCst);
    }

    pub fn mark_unused(&self) {
        self.used.store(false, Ordering::SeqCst);
    }

    pub fn is_used(&self) -> bool {
        self.used.load(Ordering::SeqCst)
    }
}

pub struct CacheData<K: IsCacheKey, T: Cacheable<K>> {
    unused: LruCache<K, Arc<CacheItem<K, T>>>,
    used: BTreeMap<K, Weak<CacheItem<K, T>>>,
}

impl<K: IsCacheKey, T: Cacheable<K>> CacheData<K, T> {
    fn get(&mut self, key: K) -> Option<Arc<CacheItem<K, T>>> {
        if let Some(e) = self.used.get(&key) {
            let found = e.clone().upgrade();
            //println!("get {:?} found some? {}", key, found.is_some());
            found
        } else {
            if let Some(e) = self.unused.get(&key) {
                //println!("get {:?} found unused", key);
                let entry = e.clone();

                drop(e);

                self.unused.pop(&key);

                entry.mark_used();

                //println!("Insert into used {:?} vs {:?}", key, entry.cache_key());

                self.used.insert(key, Arc::downgrade(&entry));

                Some(entry)
            } else {
                //println!("get {:?} not found", key);
                None
            }
        }
    }

    fn insert(&mut self, key: K, entry: &Arc<CacheItem<K, T>>) {
        entry.mark_used();

        //println!("insert to used {:?}", key);

        self.used.insert(key, Arc::downgrade(entry));
    }

    fn remove(&mut self, key: &K) {
        //println!("drop_from_cache: {:?}", key);

        if let Some(e) = self.used.get(&key) {
            if let Some(e) = e.upgrade() {
                //println!("drop_from_cache sc: {}", Arc::strong_count(&e));
                e.mark_unused();
            }
        } else {
            //println!("drop_from_cache unused: {:?}", key);
            self.unused.pop(key);
        }
    }

    fn move_to_unused(&mut self, ent: CacheItem<K, T>) {
        let key = ent.cache_key();

        if let Some(_e) = self.used.remove(&key) {
            let unused_ref = Arc::new(ent);

            unused_ref.make_unused(&Arc::downgrade(&unused_ref));

            self.unused.put(key, unused_ref);
        } else {
            //println!("move_to_unused missing entry");
        }
    }

    fn clear(&mut self) {
        self.used.clear();
        self.unused.clear();
    }
}

pub struct Cache<K: IsCacheKey, T: Cacheable<K>> {
    data: Spin<CacheData<K, T>>,
    sref: Weak<Cache<K, T>>,
}

impl<K: IsCacheKey, T: Cacheable<K>> Cache<K, T> {
    pub fn new(capacity: usize) -> Arc<Cache<K, T>> {
        Arc::new_cyclic(|me| Cache::<K, T> {
            data: Spin::new(CacheData::<K, T> {
                unused: LruCache::new(capacity),
                used: BTreeMap::new(),
            }),
            sref: me.clone(),
        })
    }

    pub fn get(&self, key: K) -> Option<Arc<CacheItem<K, T>>> {
        self.data.lock().get(key)
    }

    pub fn insert(&self, val: T) {
        let v = CacheItem::<K, T> {
            cache: self.sref.clone(),
            used: AtomicBool::new(false),
            val,
        };

        self.data.lock().insert(v.cache_key(), &Arc::new(v));
    }

    pub fn make_cached(&self, ent: &Arc<CacheItem<K, T>>) {
        ent.mark_used();

        self.data.lock().insert(ent.cache_key(), &ent);
    }

    pub fn move_to_unused(&self, ent: CacheItem<K, T>) {
        self.data.lock().move_to_unused(ent);
    }

    pub fn make_item(&self, item: T) -> Arc<CacheItem<K, T>> {
        let item = Arc::new(CacheItem::<K, T> {
            cache: self.sref.clone(),
            used: AtomicBool::new(false),
            val: item,
        });

        self.data.lock().insert(item.cache_key(), &item);

        item
    }

    pub fn make_item_cyclic(
        &self,
        factory: impl FnOnce(&Weak<CacheItem<K, T>>) -> T,
    ) -> Arc<CacheItem<K, T>> {
        let item = Arc::new_cyclic(|me| CacheItem::<K, T> {
            cache: self.sref.clone(),
            used: AtomicBool::new(false),
            val: factory(&me),
        });

        self.data.lock().insert(item.cache_key(), &item);

        item
    }

    pub fn make_item_no_cache(&self, item: T) -> Arc<CacheItem<K, T>> {
        let item = Arc::new(CacheItem::<K, T> {
            cache: Weak::default(),
            used: AtomicBool::new(false),
            val: item,
        });

        item
    }

    pub fn remove(&self, entry: &K) {
        self.data.lock().remove(&entry);
    }

    pub fn clear(&self) {
        self.data.lock().clear();
    }
}
