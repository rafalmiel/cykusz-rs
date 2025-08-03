use alloc::sync::{Arc, Weak};
use core::borrow::Borrow;
use core::fmt::Debug;
use core::hash::Hash;
use core::num::NonZeroUsize;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::{AtomicBool, Ordering};

use intrusive_collections::{LinkedList, LinkedListLink};
use lru::LruCache;

use crate::kernel::sync::{LockApi, Spin, SpinGuard};

pub trait DropHandler {
    fn handle_drop(&self, arc: Arc<Self>);

    fn debug(&self) {}
}

pub struct ArcWrap<T: DropHandler + ?Sized>(Arc<T>);
pub struct WeakWrap<T: DropHandler + ?Sized>(Weak<T>);

impl<T: DropHandler> Clone for ArcWrap<T> {
    fn clone(&self) -> Self {
        ArcWrap(self.0.clone())
    }
}

impl<T: DropHandler> Clone for WeakWrap<T> {
    fn clone(&self) -> Self {
        WeakWrap(self.0.clone())
    }
}

impl<T: DropHandler + ?Sized> Deref for ArcWrap<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DropHandler> ArcWrap<T> {
    fn from_arc(arc: Arc<T>) -> ArcWrap<T> {
        ArcWrap(arc)
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    pub fn address(&self) -> usize {
        Arc::as_ptr(&self.0) as *const u8 as usize
    }

    pub fn downgrade(ptr: &ArcWrap<T>) -> WeakWrap<T> {
        WeakWrap(Arc::downgrade(&ptr.0))
    }

    pub fn arc(&self) -> Arc<T> {
        self.0.clone()
    }
}

impl<T: DropHandler> WeakWrap<T> {
    pub fn empty() -> WeakWrap<T> {
        WeakWrap(Weak::new())
    }

    pub fn upgrade(&self) -> Option<ArcWrap<T>> {
        Some(self.0.upgrade()?.into())
    }

    pub fn strong_count(&self) -> usize {
        self.0.strong_count()
    }
}

impl<T: DropHandler> From<Arc<T>> for ArcWrap<T> {
    fn from(t: Arc<T>) -> Self {
        ArcWrap::from_arc(t)
    }
}

impl<T: DropHandler + ?Sized> Drop for ArcWrap<T> {
    fn drop(&mut self) {
        let sc = Arc::strong_count(&self.0);

        if sc == 1 {
            self.handle_drop(self.0.clone());
        }
    }
}

pub trait IsCacheKey: Hash + Ord + Borrow<Self> + Debug {}

impl<T> IsCacheKey for T where T: Hash + Ord + Borrow<Self> + Debug {}

pub trait Cacheable<K: IsCacheKey>: Sized {
    fn cache_key(&self) -> K;

    fn notify_unused(&self, _new_ref: &Weak<CacheItem<K, Self>>) {}

    fn notify_used(&self) {}

    fn deallocate(&self, _me: &CacheItem<K, Self>) {}
}

pub struct CacheItem<K: IsCacheKey, T: Cacheable<K>> {
    cache: Weak<Cache<K, T>>,
    used: AtomicBool,
    pub val: T,
    link_lock: Spin<()>,
    link: LinkedListLink,
}

impl<K: IsCacheKey, T: Cacheable<K>> CacheItem<K, T> {
    pub fn new(cache: &Weak<Cache<K, T>>, item: T) -> ArcWrap<CacheItem<K, T>> {
        Arc::new(CacheItem::<K, T> {
            cache: cache.clone(),
            used: AtomicBool::new(false),
            val: item,

            link_lock: Spin::new(()),
            link: LinkedListLink::new(),
        })
        .into()
    }

    pub fn new_cyclic(
        cache: &Weak<Cache<K, T>>,
        factory: impl FnOnce(&Weak<CacheItem<K, T>>) -> T,
    ) -> ArcWrap<CacheItem<K, T>> {
        Arc::new_cyclic(|me| CacheItem::<K, T> {
            cache: cache.clone(),
            used: AtomicBool::new(false),
            val: factory(me),

            link_lock: Spin::new(()),
            link: LinkedListLink::new(),
        })
        .into()
    }

    pub fn unlink_from_list(
        &self,
        list: &mut LinkedList<CacheItemAdapter<K, T>>,
    ) -> Option<ArcWrap<CacheItem<K, T>>> {
        let _link_lock = self.link_lock.lock();

        if self.link.is_linked() {
            let mut cur = unsafe { list.cursor_mut_from_ptr(self as *const CacheItem<K, T>) };

            if let Some(ptr) = cur.remove() {
                return Some(ptr.into());
            }
        }

        None
    }
}

impl<K: IsCacheKey, T: Cacheable<K>> ArcWrap<CacheItem<K, T>> {
    pub fn link_to_list(&self, list: &mut LinkedList<CacheItemAdapter<K, T>>) {
        let _link_lock = self.link_lock.lock();

        if self.link.is_linked() {
            panic!("linking linked list");
        }

        list.push_back(self.0.clone());
    }

    pub fn unlink_from_list(
        &self,
        list: &mut LinkedList<CacheItemAdapter<K, T>>,
    ) -> Option<ArcWrap<CacheItem<K, T>>> {
        CacheItem::<K, T>::unlink_from_list(self, list)
    }
}

unsafe impl<K: IsCacheKey, T: Cacheable<K>> Sync for CacheItem<K, T> {}

intrusive_adapter!(pub CacheItemAdapter<K, T> = Arc<CacheItem<K, T>> : CacheItem<K, T> { link: LinkedListLink } where K: IsCacheKey, T: Cacheable<K> );

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

impl<K: IsCacheKey, T: Cacheable<K>> DropHandler for CacheItem<K, T> {
    fn handle_drop(&self, arc: Arc<Self>) {
        if let Some(cache) = self.cache.upgrade() {
            if self.is_used() {
                self.mark_unused();

                cache.move_to_unused(arc.into());
            }
        }
    }

    fn debug(&self) {
        println!("{:?}", self.cache_key());
    }
}

impl<K: IsCacheKey, T: Cacheable<K>> Drop for CacheItem<K, T> {
    fn drop(&mut self) {
        self.val.deallocate(self);
    }
}

impl<K: IsCacheKey, T: Cacheable<K>> CacheItem<K, T> {
    pub fn mark_used(&self) {
        self.used.store(true, Ordering::SeqCst);

        self.notify_used();
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
    used: hashbrown::HashMap<K, Weak<CacheItem<K, T>>>,
}

impl<K: IsCacheKey, T: Cacheable<K>> CacheData<K, T> {
    fn get(&mut self, key: K) -> Option<ArcWrap<CacheItem<K, T>>> {
        if let Some(e) = self.used.get(&key) {
            let found = e.clone().upgrade();

            if let Some(f) = found {
                Some(ArcWrap::from_arc(f))
            } else {
                None
            }
        } else {
            if let Some(e) = self.unused.pop(&key) {
                e.mark_used();

                self.used.insert(key, Arc::downgrade(&e));

                Some(e.into())
            } else {
                None
            }
        }
    }

    fn insert(&mut self, key: K, entry: &ArcWrap<CacheItem<K, T>>) {
        entry.mark_used();

        self.used.insert(key, Arc::downgrade(&entry.0));
    }

    fn remove(&mut self, key: &K) {
        logln_disabled!("remove key: {:?}", key);
        self.print_stats();
        if let None = self.used.remove(key) {
            if let None = self.unused.pop(key) {
                logln_disabled!("ICache remove failed for key {:?}", key);
                self.print_stats();
            }
        }
        //if let Some(e) = self.used.get(&key) {
        //    if let Some(e) = e.upgrade() {
        //        e.mark_unused();
        //    }
        //} else {
        //    self.unused.pop(key);
        //}
    }

    fn move_to_unused(&mut self, ent: ArcWrap<CacheItem<K, T>>) -> bool {
        let key = { ent.cache_key() };

        if let Some(_e) = self.used.remove(&key) {
            self.unused.put(key, ent.0.clone());

            true
        } else {
            //println!("move_to_unused missing entry");

            false
        }
    }

    pub fn rehash(
        &mut self,
        item: &Arc<CacheItem<K, T>>,
        update: impl FnOnce(&Arc<CacheItem<K, T>>),
    ) {
        let old_key = item.cache_key();

        if let Some(v) = self.used.remove(&old_key) {
            if v.as_ptr() != Arc::as_ptr(item) {
                println!("[ WARN ] Rehash pointer mismatch.");
            }

            update(item);

            let new_key = item.cache_key();

            self.used.insert(new_key, Arc::downgrade(item));
        }
    }

    fn clear(&mut self) {
        self.used.clear();
        self.unused.clear();
    }

    fn print_stats(&self) {
        logln!("Cache usage:");
        logln!("Used count:   {}", self.used.len());
        for e in self.used.iter() {
            logln!("{:?} sc {}", e.0, e.1.strong_count());
        }
        logln!("Unused count: {}", self.unused.len());
        for e in self.unused.iter() {
            logln!("{:?} sc {}", e.0, Arc::strong_count(&e.1));
        }
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
                unused: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
                used: hashbrown::HashMap::new(),
            }),
            sref: me.clone(),
        })
    }

    pub fn get(&self, key: K) -> Option<ArcWrap<CacheItem<K, T>>> {
        self.data.lock().get(key)
    }

    pub fn insert(&self, val: T) {
        let v = CacheItem::<K, T>::new(&self.sref, val);

        self.data.lock().insert(v.cache_key(), &v);
    }

    pub fn make_cached(&self, ent: &ArcWrap<CacheItem<K, T>>) {
        ent.mark_used();

        self.data.lock().insert(ent.cache_key(), &ent);
    }

    pub fn move_to_unused(&self, ent: ArcWrap<CacheItem<K, T>>) {
        if self.data.lock().move_to_unused(ent.clone()) {
            ent.notify_unused(&Arc::downgrade(&ent));
        }
    }

    pub fn make_item_locked(
        &self,
        item: T,
        lock: &mut SpinGuard<CacheData<K, T>>,
    ) -> ArcWrap<CacheItem<K, T>> {
        let item = CacheItem::<K, T>::new(&self.sref, item);

        lock.insert(item.cache_key(), &item);

        item.notify_used();

        item
    }

    pub fn make_item(&self, item: T) -> ArcWrap<CacheItem<K, T>> {
        let item = CacheItem::<K, T>::new(&self.sref, item);

        self.data.lock().insert(item.cache_key(), &item);

        item.notify_used();

        item
    }

    pub fn make_item_cyclic(
        &self,
        factory: impl FnOnce(&Weak<CacheItem<K, T>>) -> T,
    ) -> ArcWrap<CacheItem<K, T>> {
        let item = CacheItem::<K, T>::new_cyclic(&self.sref, factory);

        self.data.lock().insert(item.cache_key(), &item);

        item
    }

    pub fn make_item_no_cache(&self, item: T) -> ArcWrap<CacheItem<K, T>> {
        let item = CacheItem::<K, T>::new(&Weak::default(), item);

        item
    }

    pub fn rehash(&self, item: &Arc<CacheItem<K, T>>, update: impl FnOnce(&Arc<CacheItem<K, T>>)) {
        self.data.lock().rehash(item, update);
    }

    pub fn remove(&self, entry: &K) {
        self.data.lock().remove(&entry);
    }

    pub fn clear(&self) {
        self.data.lock().clear();
    }

    pub fn print_stats(&self) {
        self.data.lock().print_stats();
    }
}
