//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)

use crate::{captp::CapTpSession, locator::NodeLocator};
use std::future::Future;

pub trait Netlayer {
    type Reader;
    type Writer;
    type Error;

    /// Attempt to open a new connection to the specified locator.
    fn connect<'locator>(
        &self,
        locator: &NodeLocator<'locator>,
    ) -> impl Future<Output = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>> + Send
    where
        Self: Sized;

    /// Accept a connection.
    fn accept(
        &self,
    ) -> impl Future<Output = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>> + Send
    where
        Self: Sized;

    /// Get locators pointing to this node.
    fn locators(&self) -> Vec<NodeLocator<'_>>;

    /// Get a [Stream](futures::stream::Stream) of accepted connections.
    fn stream(
        &self,
    ) -> impl futures::stream::Stream<Item = Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error>>
    where
        Self: Sized,
    {
        futures::stream::unfold(self, |nl| async move { Some((nl.accept().await, nl)) })
    }
}
