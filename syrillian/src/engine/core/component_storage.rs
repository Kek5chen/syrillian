use crate::components::{CRef, Component, ComponentId, TypedComponentId};
use log::trace;
use slotmap::HopSlotMap;
use slotmap::hop::{Values, ValuesMut};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

#[allow(unused)]
pub(crate) trait HopSlotMapUntyped<K>
where
    K: slotmap::Key + Send + 'static,
{
    fn as_dyn(&self) -> &dyn Any;
    fn as_dyn_mut(&mut self) -> &mut dyn Any;
    fn iter_comps<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Component> + 'a>;
    fn iter_comps_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut dyn Component> + 'a>;
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (K, &'a dyn Component)> + 'a>;
    fn iter_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = (K, &'a mut dyn Component)> + 'a>;
    fn get(&self, key: K) -> Option<&dyn Component>;
    fn get_mut(&mut self, key: K) -> Option<&mut dyn Component>;
    fn remove(&mut self, key: K) -> Option<Box<dyn Component>>;
}

impl<K, V> HopSlotMapUntyped<K> for HopSlotMap<K, V>
where
    K: slotmap::Key + Send + 'static,
    V: Component,
{
    fn as_dyn(&self) -> &dyn Any {
        self
    }

    fn as_dyn_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn iter_comps<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Component> + 'a> {
        Box::new(self.values().map(|v| v as &dyn Component))
    }

    fn iter_comps_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut dyn Component> + 'a> {
        Box::new(self.values_mut().map(|v| v as &mut dyn Component))
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (K, &'a dyn Component)> + 'a> {
        Box::new(self.iter().map(|(k, v)| (k, v as &dyn Component)))
    }

    fn iter_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = (K, &'a mut dyn Component)> + 'a> {
        Box::new(self.iter_mut().map(|(k, v)| (k, v as &mut dyn Component)))
    }

    fn get(&self, key: K) -> Option<&dyn Component> {
        self.get(key).map(|v| v as &dyn Component)
    }

    fn get_mut(&mut self, key: K) -> Option<&mut dyn Component> {
        self.get_mut(key).map(|v| v as &mut dyn Component)
    }

    fn remove(&mut self, key: K) -> Option<Box<dyn Component>> {
        self.remove(key).map(|v| {
            let comp: Box<dyn Component> = Box::new(v);
            comp
        })
    }
}

#[derive(Default)]
pub struct ComponentStorage {
    inner: HashMap<TypeId, Box<dyn HopSlotMapUntyped<ComponentId>>>,
    len: usize,
    pub(crate) fresh: Vec<TypedComponentId>,
    pub(crate) removed: Vec<TypedComponentId>,
}

impl ComponentStorage {
    pub(crate) fn _get_from_id(&self, tid: TypeId) -> Option<&dyn HopSlotMapUntyped<ComponentId>> {
        Some(self.inner.get(&tid)?.as_ref())
    }

    pub(crate) fn _get_mut_from_id(
        &mut self,
        tid: TypeId,
    ) -> Option<&mut dyn HopSlotMapUntyped<ComponentId>> {
        Some(self.inner.get_mut(&tid)?.as_mut())
    }

    pub(crate) fn _get<C: Component>(&self) -> Option<&HopSlotMap<ComponentId, C>> {
        let tid = TypeId::of::<C>();

        let typed = self
            ._get_from_id(tid)?
            .as_dyn()
            .downcast_ref::<HopSlotMap<ComponentId, C>>()
            .unwrap_or_else(|| {
                log::error!("The checking of a type ID resulted in an error with _get");
                std::process::exit(1);
            });

        Some(typed)
    }

    pub(crate) fn _get_mut<C: Component>(&mut self) -> Option<&mut HopSlotMap<ComponentId, C>> {
        let tid = TypeId::of::<C>();

        let typed = self
            ._get_mut_from_id(tid)?
            .as_dyn_mut()
            .downcast_mut::<HopSlotMap<ComponentId, C>>()
            .unwrap_or_else(|| {
                log::error!("The checking of a type ID resulted in an error with _get_mut");
                std::process::exit(1);
            });

        Some(typed)
    }

    pub fn get<C: Component>(&self, id: CRef<C>) -> Option<&C> {
        self._get()?.get(id.0)
    }

    pub fn get_mut<C: Component>(&mut self, id: CRef<C>) -> Option<&mut C> {
        self._get_mut()?.get_mut(id.0)
    }

    pub fn get_dyn(&self, id: TypedComponentId) -> Option<&dyn Component> {
        self._get_from_id(id.0)?.get(id.1)
    }

    pub fn get_dyn_mut(&mut self, id: TypedComponentId) -> Option<&mut dyn Component> {
        self._get_mut_from_id(id.0)?.get_mut(id.1)
    }

    pub fn values_of_type<C: Component>(&self) -> Option<Values<'_, ComponentId, C>> {
        Some(self._get()?.values())
    }

    pub fn values_mut_of_type<C: Component>(&mut self) -> Option<ValuesMut<'_, ComponentId, C>> {
        Some(self._get_mut()?.values_mut())
    }

    pub fn iter(&self) -> impl Iterator<Item = (TypedComponentId, &dyn Component)> {
        self.inner
            .iter()
            .flat_map(|(tid, store)| store.iter().map(|(k, v)| (TypedComponentId(*tid, k), v)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (TypedComponentId, &mut dyn Component)> {
        self.inner.iter_mut().flat_map(|(tid, store)| {
            store
                .iter_mut()
                .map(|(k, v)| (TypedComponentId(*tid, k), v))
        })
    }

    pub fn values(&self) -> impl Iterator<Item = &dyn Component> {
        self.inner.values().flat_map(|store| store.iter_comps())
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut dyn Component> {
        self.inner
            .values_mut()
            .flat_map(|store| store.iter_comps_mut())
    }

    pub(crate) fn map_mut<C: Component>(&mut self) -> &mut HopSlotMap<ComponentId, C> {
        let tid = TypeId::of::<C>();
        self.inner
            .entry(tid)
            .or_insert_with(|| Box::new(HopSlotMap::<ComponentId, C>::with_key()))
            .as_dyn_mut()
            .downcast_mut()
            .unwrap_or_else(|| {
                log::error!("An error was encountered while trying to get a mutable ref to an HopSlotMap for the components IDs");
                std::process::exit(1);
            })
    }

    pub(crate) fn add<C: Component>(&mut self, component: C) -> CRef<C> {
        let comp: CRef<C> = CRef(self.map_mut().insert(component), PhantomData);
        self.len += 1;
        self.fresh.push(comp.into());
        comp
    }

    pub(crate) fn remove(&mut self, ctid: TypedComponentId) {
        trace!("Removed component");

        let Some(map) = self._get_mut_from_id(ctid.0) else {
            // already empty
            return;
        };

        let comp = map.remove(ctid.1);
        debug_assert!(
            comp.is_some(),
            "Component wasn't found despite still being on owned by a game object."
        );
        self.removed.push(ctid);

        debug_assert_ne!(self.len, 0);

        self.len = self.len.saturating_sub(1);
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}
