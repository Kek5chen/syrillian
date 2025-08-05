use crate::engine::assets::key::AssetKey;
use crate::engine::assets::{HShader, H};
use dashmap::iter::Iter;
use dashmap::mapref::one::Ref as MapRef;
use dashmap::mapref::one::RefMut as MapRefMut;
use dashmap::DashMap;
use log::{trace, warn};
use std::fmt::{Debug, Display, Formatter};
use std::mem;
use std::sync::RwLock;

type Ref<'a, T> = MapRef<'a, AssetKey, T>;
type RefMut<'a, T> = MapRefMut<'a, AssetKey, T>;

pub struct Store<T: StoreType> {
    data: DashMap<AssetKey, T>,
    next_id: RwLock<u32>,
    dirty: RwLock<Vec<AssetKey>>,
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

    fn store<S: AsRef<Store<Self>>>(self, store: &S) -> H<Self> {
        store.as_ref().add(self)
    }
    fn is_builtin(handle: H<Self>) -> bool;
}

pub trait StoreTypeFallback: StoreType {
    fn fallback() -> H<Self>;
}

pub trait StoreTypeName: StoreType {
    fn name(&self) -> &str;
}

pub enum HandleName<T: StoreType> {
    Static(&'static str),
    Id(H<T>),
}

impl<T: StoreType> Store<T> {
    pub fn empty() -> Self {
        Self {
            data: DashMap::new(),
            next_id: RwLock::new(0),
            dirty: RwLock::default(),
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

impl<T: StoreType> Display for HandleName<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleName::Static(s) => write!(f, "\"{s}\"",),
            HandleName::Id(id) => write!(f, "#{id}"),
        }
    }
}

impl<T: StoreType> Store<T> {
    fn next_id(&self) -> H<T> {
        let mut id_lock = self.next_id.write().unwrap();
        let id = H::new(*id_lock);
        *id_lock += 1;
        id
    }

    pub fn add<T2: Into<T>>(&self, elem: T2) -> H<T> {
        let id = self.next_id();
        self.data.insert(id.into(), elem.into());

        trace!("[{} Store] Added element: {}", T::name(), T::ident_fmt(id));

        id
    }

    pub fn try_get(&self, h: H<T>) -> Option<Ref<'_, T>> {
        self.data.get(&h.into()).or_else(|| {
            warn!(
                "[{} Store] Invalid Reference: h={} not found",
                T::name(),
                T::ident_fmt(h)
            );
            None
        })
    }

    pub fn try_get_mut(&mut self, h: H<T>) -> Option<RefMut<'_, T>> {
        self._try_get_mut(h)
    }

    fn _try_get_mut(&self, h: H<T>) -> Option<RefMut<'_, T>> {
        self.data
            .get_mut(&h.into())
            .or_else(|| {
                warn!(
                    "[{} Store] Invalid Reference: h={} not found",
                    T::name(),
                    T::ident_fmt(h)
                );
                None
            })
            .and_then(|v| {
                self.set_dirty(h.into());
                Some(v)
            })
    }

    fn set_dirty(&self, h: AssetKey) {
        let mut dirty_store = self.dirty.write().expect("Deadlock in Asset Store");
        if !dirty_store.contains(&h.into()) {
            trace!("Set {} {} dirty", T::name(), T::ident(h.into()));
            dirty_store.push(h.into());
        }
    }

    pub(crate) fn pop_dirty(&self) -> Vec<AssetKey> {
        let mut dirty_store = self.dirty.write().expect("Deadlock in Asset Store");
        let mut swap_store = Vec::new();
        mem::swap::<Vec<AssetKey>>(dirty_store.as_mut(), swap_store.as_mut());

        swap_store
    }

    pub fn remove(&self, h: H<T>) -> Option<T> {
        if h.is_builtin() {
            return None;
        }
        let key = h.into();
        let item = self.data.remove(&key);
        self.set_dirty(key);
        Some(item?.1)
    }

    pub fn items(&self) -> Iter<AssetKey, T> {
        self.data.iter()
    }
}

impl<T: StoreTypeFallback> Store<T> {
    pub fn get(&self, h: H<T>) -> Ref<'_, T> {
        if !self.data.contains_key(&h.into()) {
            let fallback = self.try_get(T::fallback());
            match fallback {
                Some(elem) => elem,
                None => unreachable!("Fallback items should always be populated"),
            }
        } else {
            let data = self.data.get(&h.into());
            match data {
                Some(elem) => elem,
                None => unreachable!("Item was checked previously"),
            }
        }
    }

    pub fn get_mut(&self, h: H<T>) -> RefMut<'_, T> {
        if !self.data.contains_key(&h.into()) {
            let fallback = self._try_get_mut(T::fallback());
            match fallback {
                Some(elem) => elem,
                None => unreachable!("Fallback items should always be populated"),
            }
        } else {
            let data = self.data.get_mut(&h.into());
            self.set_dirty(h.into());
            match data {
                Some(elem) => elem,
                None => unreachable!("Item was checked previously"),
            }
        }
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
    ($store:ident, $expected_id:path, $elem:expr) => {
        $store.add($elem);
    };
}

#[macro_export]
macro_rules! store_add_checked_many {
    ($store:ident, $( $expected_id:path => $elem:expr ),+ $(,)?) => {
        $( store_add_checked!($store, $expected_id, $elem); )*
    }
}
