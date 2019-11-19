use super::one::RefMut;
use crate::util;
use dashmap_shard::HashMap;
use fxhash::FxBuildHasher;
use parking_lot::RwLockWriteGuard;
use std::hash::Hash;
use std::mem;
use std::ptr;

pub enum Entry<'a, K: Eq + Hash, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K: Eq + Hash, V> Entry<'a, K, V> {
    #[inline(always)]
    pub fn and_modify(self, f: impl FnOnce(&mut V)) -> Self {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }

            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    #[inline(always)]
    pub fn key(&self) -> &K {
        match *self {
            Entry::Occupied(ref entry) => entry.key(),
            Entry::Vacant(ref entry) => entry.key(),
        }
    }

    #[inline(always)]
    pub fn or_default(self) -> RefMut<'a, K, V>
    where
        V: Default,
    {
        match self {
            Entry::Occupied(entry) => entry.into_ref(),
            Entry::Vacant(entry) => entry.insert(V::default()),
        }
    }

    #[inline(always)]
    pub fn or_insert(self, value: V) -> RefMut<'a, K, V> {
        match self {
            Entry::Occupied(entry) => entry.into_ref(),
            Entry::Vacant(entry) => entry.insert(value),
        }
    }

    #[inline(always)]
    pub fn or_insert_with(self, value: impl FnOnce() -> V) -> RefMut<'a, K, V> {
        match self {
            Entry::Occupied(entry) => entry.into_ref(),
            Entry::Vacant(entry) => entry.insert(value()),
        }
    }
}

pub struct VacantEntry<'a, K: Eq + Hash, V> {
    shard: RwLockWriteGuard<'a, HashMap<K, V, FxBuildHasher>>,
    key: K,
}

impl<'a, K: Eq + Hash, V> VacantEntry<'a, K, V> {
    #[inline(always)]
    pub fn new(shard: RwLockWriteGuard<'a, HashMap<K, V, FxBuildHasher>>, key: K) -> Self {
        Self { shard, key }
    }

    #[inline(always)]
    pub fn insert(mut self, value: V) -> RefMut<'a, K, V> {
        unsafe {
            let c: K = ptr::read(&self.key);
            self.shard.insert(self.key, value);
            let (k, v) = self.shard.get_key_value(&c).unwrap();
            let k = util::change_lifetime_const(k);
            let v = util::change_lifetime_mut(util::to_mut(v));
            let r = RefMut::new(self.shard, k, v);
            mem::forget(c);
            r
        }
    }

    #[inline(always)]
    pub fn into_key(self) -> K {
        self.key
    }

    #[inline(always)]
    pub fn key(&self) -> &K {
        &self.key
    }
}

pub struct OccupiedEntry<'a, K: Eq + Hash, V> {
    shard: RwLockWriteGuard<'a, HashMap<K, V, FxBuildHasher>>,
    elem: (&'a K, &'a mut V),
    key: Option<K>,
}

impl<'a, K: Eq + Hash, V> OccupiedEntry<'a, K, V> {
    #[inline(always)]
    pub fn new(
        shard: RwLockWriteGuard<'a, HashMap<K, V, FxBuildHasher>>,
        key: Option<K>,
        elem: (&'a K, &'a mut V),
    ) -> Self {
        Self { shard, elem, key }
    }

    #[inline(always)]
    pub fn get(&self) -> &V {
        self.elem.1
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut V {
        self.elem.1
    }

    #[inline(always)]
    pub fn insert(&mut self, value: V) -> V {
        mem::replace(self.elem.1, value)
    }

    #[inline(always)]
    pub fn into_ref(self) -> RefMut<'a, K, V> {
        RefMut::new(self.shard, self.elem.0, self.elem.1)
    }

    #[inline(always)]
    pub fn key(&self) -> &K {
        self.elem.0
    }

    #[inline(always)]
    pub fn remove(mut self) -> V {
        self.shard.remove(self.elem.0).unwrap()
    }

    #[inline(always)]
    pub fn remove_entry(mut self) -> (K, V) {
        self.shard.remove_entry(self.elem.0).unwrap()
    }

    #[inline(always)]
    pub fn replace_entry(mut self, value: V) -> (K, V) {
        let nk = self.key.unwrap();
        let p = self.shard.remove_entry(self.elem.0).unwrap();
        self.shard.insert(nk, value);
        p
    }

    #[inline(always)]
    pub fn replace_key(self) -> K {
        let r = unsafe { util::to_mut(self.elem.0) };
        mem::replace(r, self.key.unwrap())
    }
}
