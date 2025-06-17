use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BiMap<K, V> {
    forward: HashMap<K, V>,
    backward: HashMap<V, K>,
}

impl<K: std::hash::Hash + Eq + Clone, V: std::hash::Hash + Eq + Clone> BiMap<K, V> {
    pub fn new() -> Self {
        BiMap {
            forward: HashMap::new(),
            backward: HashMap::new(),
        }
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.forward.insert(k.clone(), v.clone());
        self.backward.insert(v, k);
    }

    pub fn get_by_key(&self, k: &K) -> Option<&V> {
        self.forward.get(k)
    }

    pub fn get_by_value(&self, v: &V) -> Option<&K> {
        self.backward.get(v)
    }

    pub fn remove_by_key(&mut self, k: &K) {
        if let Some(v) = self.forward.remove(k) {
            self.backward.remove(&v);
        }
    }

    pub fn remove_by_value(&mut self, v: &V) {
        if let Some(k) = self.backward.remove(v) {
            self.forward.remove(&k);
        }
    }
}
