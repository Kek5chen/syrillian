use crate::engine::assets::key::AssetKey;
use crate::engine::assets::{H, HShader};
use dashmap::DashMap;
use dashmap::mapref::one::Ref as MapRef;
use dashmap::mapref::one::RefMut as MapRefMut;
use log::{trace, warn};
use std::fmt::{Debug, Display, Formatter};
use std::sync::RwLock;

pub struct Store<T: StoreType> {
    data: DashMap<AssetKey, T>,
    next_id: RwLock<u32>,
}

impl<T: StoreType> Store<T> {
    pub fn empty() -> Self {
        Self {
            data: DashMap::new(),
            next_id: RwLock::new(0),
        }
    }
}

impl<T: StoreDefaults> Store<T> {
    pub fn populated() -> Self {
        let mut store = Self::empty();
        store.populate();
        store
    }

    pub fn populate(&mut self) {
        T::populate(self);
    }
}

pub enum HandleName<T: StoreType> {
    Static(&'static str),
    Id(H<T>),
}

impl<T: StoreType> Display for HandleName<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleName::Static(s) => write!(f, "\"{s}\"",),
            HandleName::Id(id) => write!(f, "#{id}"),
        }
    }
}

pub trait StoreDefaults: StoreType {
    fn populate(store: &mut Store<Self>);
}

pub trait StoreType: Sized + Debug {
    fn name() -> &'static str;
    fn ident_fmt(handle: H<Self>) -> HandleName<Self>;
    fn ident(handle: H<Self>) -> String {
        match Self::ident_fmt(handle) {
            HandleName::Static(name) => name.to_string(),
            HandleName::Id(id) => format!("{} #{id}", Self::name()),
        }
    }
}

pub trait StoreTypeFallback: StoreType {
    fn fallback() -> H<Self>;
}

pub trait StoreTypeName: StoreType {
    fn name(&self) -> &str;
}

type Ref<'a, T> = MapRef<'a, AssetKey, T>;
type RefMut<'a, T> = MapRefMut<'a, AssetKey, T>;

impl<T: StoreType> Store<T> {
    fn next_id(&self) -> H<T> {
        let mut id_lock = self.next_id.write().unwrap();
        let id = H::new(*id_lock);
        *id_lock += 1;
        id
    }
    pub fn add(&self, elem: T) -> H<T> {
        let id = self.next_id();
        self.data.insert(id.into(), elem);

        trace!("[{} Store] Added element: {}", T::name(), T::ident_fmt(id));

        id
    }

    pub fn _try_get_mut(&self, h: H<T>) -> Option<RefMut<'_, T>> {
        self.data.get_mut(&h.into()).or_else(|| {
            warn!(
                "[{} Store] Invalid Reference: h={} not found",
                T::name(),
                T::ident_fmt(h)
            );
            None
        })
    }

    pub fn try_get(&self, h: H<T>) -> Option<Ref<'_, T>> {
        self._try_get_mut(h).map(RefMut::downgrade)
    }

    pub fn try_get_mut(&mut self, h: H<T>) -> Option<RefMut<'_, T>> {
        self._try_get_mut(h)
    }
}

impl<T: StoreTypeFallback> Store<T> {
    fn _get_mut(&self, h: H<T>) -> RefMut<'_, T> {
        if !self.data.contains_key(&h.into()) {
            let fallback = self._try_get_mut(T::fallback());
            match fallback {
                Some(elem) => elem,
                None => unreachable!("Fallback items should always be populated"),
            }
        } else {
            let data = self.data.get_mut(&h.into());
            match data {
                Some(elem) => elem,
                None => unreachable!("Item was checked previously"),
            }
        }
    }
    pub fn get(&self, h: H<T>) -> Ref<'_, T> {
        self._get_mut(h).downgrade()
    }

    pub fn get_mut(&self, h: H<T>) -> RefMut<'_, T> {
        self._get_mut(h)
    }
}

impl<T: StoreTypeName> Store<T> {
    pub fn find_by_name(&self, name: &str) -> Option<HShader> {
        self.data
            .iter()
            .find(|e| e.value().name() == name)
            .map(|e| e.key().clone().into())
    }
}

#[macro_export]
#[cfg(debug_assertions)]
macro_rules! store_add_checked {
    ($store:ident, $expected_id:path, $elem:expr) => {
        let id = $store.add($elem);
        assert_eq!(id.id(), $expected_id);
    };
}

#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! store_add_checked {
    ($store:ident, $expected_id:ident, $elem:expr) => {
        $store.add($elem);
    };
}
