use syrup::Sequence;

use super::{FetchResolver, GenericResolver};
use crate::captp::msg::DescExport;

#[derive(Clone)]
pub enum BootstrapEvent {
    Fetch {
        swiss: Vec<u8>,
        resolver: FetchResolver,
    },
}

impl std::fmt::Debug for BootstrapEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fetch { swiss, resolver } => f
                .debug_struct("Fetch")
                .field("swiss", &crate::hash(&swiss))
                .field("resolver", resolver)
                .finish(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Delivery<'args> {
    DeliverOnly {
        to_desc: DescExport,
        args: Sequence<'args>,
    },
    Deliver {
        to_desc: DescExport,
        args: Sequence<'args>,
        resolver: GenericResolver,
    },
}

impl<'args> Delivery<'args> {
    pub fn position(&self) -> u64 {
        match self {
            Delivery::DeliverOnly { to_desc, .. } | Delivery::Deliver { to_desc, .. } => {
                to_desc.position
            }
        }
    }
}

#[derive(Clone)]
pub enum Event {
    Bootstrap(BootstrapEvent),
    // Delivery(Delivery),
    Abort(String),
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bootstrap(arg0) => f.debug_tuple("Bootstrap").field(arg0).finish(),
            Self::Abort(arg0) => f.debug_tuple("Abort").field(arg0).finish(),
        }
    }
}
