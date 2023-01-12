use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Mutex;

use once_cell::sync::OnceCell;

pub struct StaticHashCache<T: 'static + Eq + Hash>(OnceCell<Mutex<HashSet<&'static T>>>);

impl<T: 'static + Eq + Hash> StaticHashCache<T> {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    pub fn intern(&self, value: T) -> &'static T {
        let cache = self.0.get_or_init(|| Mutex::new(HashSet::new()));
        let mut cache = cache.lock().unwrap();

        cache.get(&value).cloned().unwrap_or_else(|| {
            let interned = Box::leak(Box::new(value));
            cache.insert(interned);
            interned
        })
    }
}
