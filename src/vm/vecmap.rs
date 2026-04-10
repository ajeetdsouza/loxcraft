use std::hash::{BuildHasher, BuildHasherDefault, Hash};
use std::slice;

use hashbrown::{HashMap, hash_map};
use rustc_hash::FxHasher;

/// Max entries before spilling from Vec to HashMap.
const VEC_MAP_SPILL: usize = 16;

/// Type alias matching `HashMap<K, V, BuildHasherDefault<FxHasher>>`.
pub type FxVecMap<K, V> = VecMap<K, V, BuildHasherDefault<FxHasher>>;

/// A map that uses a `Vec` for linear scan when small (≤ [`VEC_MAP_SPILL`]
/// entries) and spills to a `HashMap` when larger. Empty maps allocate nothing.
#[derive(Clone, Debug)]
pub enum VecMap<K, V, S = BuildHasherDefault<FxHasher>> {
    Vec(Vec<(K, V)>),
    Map(HashMap<K, V, S>),
}

impl<K, V, S> Default for VecMap<K, V, S> {
    fn default() -> Self {
        Self::Vec(Vec::new())
    }
}

impl<K: Eq + Hash, V, S: BuildHasher + Default> VecMap<K, V, S> {
    pub fn get(&self, key: &K) -> Option<&V> {
        match self {
            Self::Vec(vec) => vec.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            Self::Map(map) => map.get(key),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        match self {
            Self::Vec(vec) => {
                if let Some((_, v)) = vec.iter_mut().find(|(k, _)| k == &key) {
                    *v = value;
                    return;
                }
                if vec.len() < VEC_MAP_SPILL {
                    vec.push((key, value));
                    return;
                }
                // Spill to HashMap.
                let Self::Vec(vec) = std::mem::take(self) else { unreachable!() };
                let map = vec.into_iter().chain([(key, value)]).collect();
                *self = Self::Map(map);
            }
            Self::Map(map) => {
                map.insert(key, value);
            }
        }
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn iter(&self) -> VecMapIter<'_, K, V> {
        match self {
            Self::Vec(vec) => VecMapIter::Vec(vec.iter()),
            Self::Map(map) => VecMapIter::Map(map.iter()),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Vec(vec) => vec.len(),
            Self::Map(map) => map.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub enum VecMapIter<'a, K, V> {
    Vec(slice::Iter<'a, (K, V)>),
    Map(hash_map::Iter<'a, K, V>),
}

impl<'a, K, V> Iterator for VecMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Vec(it) => it.next().map(|(k, v)| (k, v)),
            Self::Map(it) => it.next(),
        }
    }
}

impl<'a, K, V, S> IntoIterator for &'a VecMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher + Default,
{
    type Item = (&'a K, &'a V);
    type IntoIter = VecMapIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
