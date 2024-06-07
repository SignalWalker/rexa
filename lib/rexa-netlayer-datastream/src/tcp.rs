use std::{borrow::Cow, collections::HashMap, net::SocketAddr};

use futures::FutureExt;
use rexa::locator::NodeLocator;
use syrup::symbol;

use super::{AsyncDataStream, AsyncStreamListener};

pub type TcpIpNetlayer = super::DataStreamNetlayer<tokio::net::TcpListener>;

impl TcpIpNetlayer {
    pub fn addresses(&self) -> impl Iterator<Item = SocketAddr> + '_ {
        self.listeners
            .iter()
            .map(|listener| listener.local_addr().unwrap())
    }
}

impl AsyncStreamListener for tokio::net::TcpListener {
    const TRANSPORT: &'static str = "tcpip";
    /// FIX :: [permit impl trait in type alias](https://github.com/rust-lang/rust/issues/63063)
    type AddressInput<'addr> = &'addr SocketAddr;
    type AddressOutput = std::net::SocketAddr;
    type Error = std::io::Error;
    type Stream = tokio::net::TcpStream;

    async fn bind(addr: Self::AddressInput<'_>) -> Result<Self, Self::Error> {
        tokio::net::TcpListener::bind(addr).await
    }

    fn accept(
        &self,
    ) -> impl std::future::Future<Output = Result<(Self::Stream, SocketAddr), Self::Error>> + Send + Unpin
    {
        tokio::net::TcpListener::accept(self).boxed()
    }

    fn local_addr(&self) -> Result<Self::AddressOutput, Self::Error> {
        tokio::net::TcpListener::local_addr(self)
    }

    fn locator(&self) -> Result<NodeLocator<'static>, Self::Error> {
        let addr = self.local_addr()?;
        Ok(NodeLocator {
            designator: Cow::Owned(addr.ip().to_string()),
            transport: Cow::Borrowed(Self::TRANSPORT),
            hints: HashMap::from_iter([(symbol!["port"], Cow::Owned(addr.port().to_string()))]),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TcpConnectError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("expected connect address to specify port")]
    MissingPort,
    #[error(transparent)]
    ParsePort(#[from] std::num::ParseIntError),
}

impl AsyncDataStream for tokio::net::TcpStream {
    type ReadHalf = tokio::net::tcp::OwnedReadHalf;
    type WriteHalf = tokio::net::tcp::OwnedWriteHalf;
    type Error = TcpConnectError;

    async fn connect<'loc>(addr: &NodeLocator<'loc>) -> Result<Self, Self::Error> {
        tokio::net::TcpStream::connect((
            &*addr.designator,
            addr.hint_into("port").ok_or(<Self::Error>::MissingPort)??,
        ))
        .await
        .map_err(From::from)
    }

    fn split(self) -> (Self::ReadHalf, Self::WriteHalf) {
        tokio::net::TcpStream::into_split(self)
    }
}
