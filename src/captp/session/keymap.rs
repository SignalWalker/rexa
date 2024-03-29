use dashmap::DashMap;
use std::sync::atomic::AtomicU64;

#[must_use]
pub(crate) struct Reservation<'map, Value> {
    key: u64,
    map: &'map DashMap<u64, Value>,
}

impl<'m, V> Reservation<'m, V> {
    #[inline]
    pub(crate) fn key(&self) -> u64 {
        self.key
    }

    pub(crate) fn finalize(self, value: V) -> u64 {
        self.map.insert(self.key, value);
        self.key
    }
}

#[derive(Debug, Default)]
pub(crate) struct KeyMap<Value> {
    map: DashMap<u64, Value>,
    current_key: AtomicU64,
}

impl<V> KeyMap<V> {
    pub(crate) fn with_initial(initial: u64) -> Self {
        Self {
            map: DashMap::new(),
            current_key: initial.into(),
        }
    }
    fn next_key(&self) -> u64 {
        use std::sync::atomic::Ordering;
        self.current_key.fetch_add(1, Ordering::AcqRel)
    }
    pub(crate) fn reserve(&self) -> Reservation<'_, V> {
        Reservation {
            key: self.next_key(),
            map: &self.map,
        }
    }

    pub(crate) fn push(&self, value: V) -> u64 {
        let key = self.next_key();
        self.map.insert(key, value);
        key
    }
    pub(crate) fn remove(&self, key: u64) -> Option<V> {
        self.map.remove(&key).map(|(_, v)| v)
    }

    #[tracing::instrument(level = tracing::Level::TRACE, skip(self))]
    pub(crate) fn get<'s>(&'s self, key: &u64) -> Option<dashmap::mapref::one::Ref<'s, u64, V>> {
        self.map.get(key)
    }

    #[tracing::instrument(level = tracing::Level::TRACE, skip(self))]
    pub(crate) fn get_mut<'s>(
        &'s self,
        key: &u64,
    ) -> Option<dashmap::mapref::one::RefMut<'s, u64, V>> {
        self.map.get_mut(key)
    }
}
