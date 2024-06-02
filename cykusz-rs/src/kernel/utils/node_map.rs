use crate::kernel::fs::inode::INode;
use alloc::sync::Arc;
use hashbrown::HashMap;
use syscall_defs::SyscallError;

pub trait NodeMapItem: INode {
    fn new(_key: Option<(usize, usize)>) -> Arc<Self>;
    fn key(&self) -> Option<(usize, usize)>;
}

pub struct NodeMap<T: NodeMapItem> {
    map: HashMap<(usize, usize), Arc<T>>,
}

impl<T: NodeMapItem> NodeMap<T> {
    pub fn new() -> NodeMap<T> {
        NodeMap {
            map: HashMap::new(),
        }
    }

    fn get_key(inode: &Arc<dyn INode>) -> Option<(usize, usize)> {
        Some((
            Arc::as_ptr(&inode.fs()?.upgrade()?.device()) as *const () as usize,
            inode.id().unwrap(),
        ))
    }

    pub fn insert(&mut self, inode: &Arc<dyn INode>, node: &Arc<T>) -> Result<(), SyscallError> {
        let key = Self::get_key(inode).ok_or(SyscallError::EINVAL)?;

        self.map.insert(key, node.clone());

        Ok(())
    }

    pub fn get(&mut self, inode: &Arc<dyn INode>) -> Option<Arc<T>> {
        let key = Self::get_key(inode)?;

        self.map.get(&key).cloned()
    }

    pub fn get_or_insert_default(&mut self, inode: &Arc<dyn INode>) -> Option<Arc<T>> {
        let key = Self::get_key(inode)?;

        match self.map.try_insert(key, T::new(Some(key))) {
            Ok(v) => {
                logln!("getting new node -> created: {:?}", key);
                Some(v.clone())
            }
            Err(e) => {
                logln!("getting new node -> returned: {:?}", key);
                Some(e.entry.get().clone())
            }
        }
    }

    pub fn remove(&mut self, inode: &Arc<T>) {
        let key = inode.key();
        if let Some(k) = key {
            logln!("remove node {:?}", k);
            self.map.remove(&k);
        }
    }
}
