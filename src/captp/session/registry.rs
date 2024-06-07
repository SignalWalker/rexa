use crate::captp::object::DeliverySender;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct SwissRegistry {
    map: dashmap::DashMap<Vec<u8>, DeliverySender<'static>>,
}

impl SwissRegistry {
    pub fn new() -> Arc<Self> {
        Arc::default()
    }

    pub fn insert(
        &self,
        key: Vec<u8>,
        value: DeliverySender<'static>,
    ) -> Option<DeliverySender<'static>> {
        self.map.insert(key, value)
    }

    pub fn get<'s>(
        &'s self,
        swiss: &[u8],
    ) -> Option<dashmap::mapref::one::Ref<'s, Vec<u8>, DeliverySender<'static>>> {
        self.map.get(swiss)
    }

    pub fn remove(&self, swiss: &[u8]) -> Option<DeliverySender<'static>> {
        self.map.remove(swiss).map(|(_, v)| v)
    }
}
