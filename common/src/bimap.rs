use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::collections::hash_map;
use std::iter::IntoIterator;

#[derive(Debug, Clone)]
pub struct BiMap<K, V> {
    forward: HashMap<K, V>,
    backward: HashMap<V, K>,
}

impl<K, V> BiMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: std::hash::Hash + Eq + Clone,
{
    pub fn new() -> Self {
        BiMap {
            forward: HashMap::new(),
            backward: HashMap::new(),
        }
    }

    pub fn insert(&mut self, k: K, v: V) {
        // Elimina cualquier valor anterior asociado a la clave
        if let Some(old_v) = self.forward.get(&k) {
            self.backward.remove(old_v);
        }
        // Elimina cualquier clave anterior asociada al valor
        if let Some(old_k) = self.backward.get(&v) {
            self.forward.remove(old_k);
        }
        self.forward.insert(k.clone(), v.clone());
        self.backward.insert(v, k);
    }

    pub fn get_by_key(&self, k: &K) -> Option<&V> {
        self.forward.get(k)
    }

    pub fn get_by_value(&self, v: &V) -> Option<&K> {
        self.backward.get(v)
    }

    pub fn remove_by_key(&mut self, k: &K) -> Option<K> {
        if let Some(v) = self.forward.remove(k) {
            return self.backward.remove(&v);
        }
        None
    }

    pub fn remove_by_value(&mut self, v: &V) {
        if let Some(k) = self.backward.remove(v) {
            self.forward.remove(&k);
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.forward.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.forward.values()
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.forward.contains_key(k)
    }

    pub fn contains_value(&self, v: &V) -> bool {
        self.backward.contains_key(v)
    }
}

// Manual Serialize
impl<K, V> Serialize for BiMap<K, V>
where
    K: std::hash::Hash + Eq + Clone + Serialize,
    V: std::hash::Hash + Eq + Clone + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.forward.serialize(serializer)
    }
}

// Manual Deserialize
impl<'de, K, V> Deserialize<'de> for BiMap<K, V>
where
    K: std::hash::Hash + Eq + Clone + Deserialize<'de>,
    V: std::hash::Hash + Eq + Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let forward = HashMap::<K, V>::deserialize(deserializer)?;
        let mut backward = HashMap::with_capacity(forward.len());
        for (k, v) in &forward {
            backward.insert(v.clone(), k.clone());
        }
        Ok(BiMap { forward, backward })
    }
}

impl<K, V> IntoIterator for BiMap<K, V> {
    type Item = (K, V);
    type IntoIter = hash_map::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.forward.into_iter()
    }
}

impl<'a, K, V> IntoIterator for &'a BiMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = hash_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.forward.iter()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: std::hash::Hash + Eq + Clone> Default for BiMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
