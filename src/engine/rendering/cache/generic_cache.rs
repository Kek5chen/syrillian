use crate::engine::assets::generic_store::{Store, StoreDefaults, StoreType};
use crate::engine::assets::{AssetKey, H, StoreTypeFallback};
use crate::engine::rendering::cache::AssetCache;
use dashmap::DashMap;
use log::warn;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use wgpu::{Device, Queue};

type Slot<T> = Arc<<T as CacheType>::Hot>;

pub struct Cache<T: CacheType> {
    data: DashMap<AssetKey, Slot<T>>,
    cache_misses: RwLock<AtomicUsize>,

    store: Arc<Store<T>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

pub trait CacheType: Sized + StoreType + StoreDefaults {
    type Hot;
    fn upload(&self, device: &Device, queue: &Queue, cache: &AssetCache) -> Self::Hot;
}

impl<T: CacheType + StoreTypeFallback> Cache<T> {
    pub fn get(&self, h: H<T>, cache: &AssetCache) -> Arc<T::Hot> {
        self.data
            .entry(h.into())
            .or_insert_with(|| Arc::new(self.refresh_item(h, &self.device, &self.queue, cache)))
            .clone()
    }

    fn refresh_item(&self, h: H<T>, device: &Device, queue: &Queue, cache: &AssetCache) -> T::Hot {
        let cold = self.store.get(h);

        let misses_atom = self.cache_misses.write().unwrap();
        let misses = misses_atom.load(Ordering::Acquire) + 1;
        misses_atom.fetch_add(1, Ordering::Relaxed);

        if misses % 1000 == 0 {
            warn!(
                "[{} Cache] Invalid Handle: {}, Misses: {}",
                T::name(),
                T::ident_fmt(h),
                misses
            );
        }

        cold.upload(device, queue, cache)
    }
}

impl<T: CacheType> Cache<T> {
    pub fn new(store: Arc<Store<T>>, device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Cache {
            data: DashMap::new(),
            cache_misses: RwLock::new(AtomicUsize::new(0)),
            store,
            device,
            queue,
        }
    }
    
    pub fn try_get(&self, h: H<T>, cache: &AssetCache) -> Option<Arc<T::Hot>> {
        self.data
            .entry(h.into())
            .or_try_insert_with(|| {
                self.try_refresh_item(h, &self.device, &self.queue, cache)
                    .map(Arc::new)
            })
            .ok()
            .map(|h| h.clone())
    }

    pub fn refresh_dirty(&self) -> usize {
        let dirty = self.store.pop_dirty();

        for asset in &dirty {
            self.data.remove(asset);
        }

        dirty.len()
    }

    fn try_refresh_item(
        &self,
        h: H<T>,
        device: &Device,
        queue: &Queue,
        cache: &AssetCache,
    ) -> Result<T::Hot, ()> {
        let cold = self.store.try_get(h).ok_or(())?;

        let misses_atom = self.cache_misses.write().unwrap();
        let misses = misses_atom.load(Ordering::Acquire) + 1;
        misses_atom.fetch_add(1, Ordering::Relaxed);

        if misses % 1000 == 0 {
            warn!(
                "[{} Cache] Invalid Handle: {}, Misses: {}",
                T::name(),
                T::ident_fmt(h),
                misses
            );
        }

        Ok(cold.upload(device, queue, cache))
    }
}
