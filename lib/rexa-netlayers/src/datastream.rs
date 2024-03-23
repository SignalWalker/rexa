use rexa::{
    async_compat::RwLock,
    captp::{CapTpSession, CapTpSessionManager},
    locator::NodeLocator,
    netlayer::Netlayer,
};

#[cfg(feature = "datastream-tcp")]
mod tcp;
#[cfg(feature = "datastream-tcp")]
pub use tcp::*;

#[cfg(all(feature = "datastream-unix", target_family = "unix"))]
mod unix;
#[cfg(all(feature = "datastream-unix", target_family = "unix"))]
pub use unix::*;

pub trait AsyncStreamListener: Sized {
    const TRANSPORT: &'static str;
    type AddressInput<'addr>;
    type AddressOutput;
    type Error;
    type Stream: AsyncDataStream;
    fn bind(
        addrs: Self::AddressInput<'_>,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>>;
    fn accept(
        &self,
    ) -> impl std::future::Future<Output = Result<(Self::Stream, Self::AddressOutput), Self::Error>>
           + std::marker::Send
           + Unpin;
    fn local_addr(&self) -> Result<Self::AddressOutput, Self::Error>;
    fn locator(&self) -> Result<NodeLocator, Self::Error>;
}

pub trait AsyncDataStream: Sized {
    type ReadHalf;
    type WriteHalf;
    type Error;
    fn connect(
        addr: &NodeLocator,
    ) -> impl std::future::Future<Output = Result<Self, Self::Error>> + std::marker::Send;
    fn split(self) -> (Self::ReadHalf, Self::WriteHalf);
}

#[derive(Debug, thiserror::Error)]
pub enum Error<Listener, Stream> {
    #[error(transparent)]
    Listener(Listener),
    #[error(transparent)]
    Stream(Stream),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct DataStreamNetlayer<Listener: AsyncStreamListener> {
    listeners: Vec<Listener>,
    manager: RwLock<
        CapTpSessionManager<
            <Listener::Stream as AsyncDataStream>::ReadHalf,
            <Listener::Stream as AsyncDataStream>::WriteHalf,
        >,
    >,
}

impl<Listener: AsyncStreamListener> Netlayer for DataStreamNetlayer<Listener>
where
    Listener::Stream: AsyncDataStream,
    Listener::Error: std::error::Error,
    <Listener::Stream as AsyncDataStream>::ReadHalf: rexa::async_compat::AsyncRead + Unpin + Send,
    <Listener::Stream as AsyncDataStream>::WriteHalf: rexa::async_compat::AsyncWrite + Unpin + Send,
    <Listener::Stream as AsyncDataStream>::Error: std::error::Error,
    Self: Sync,
{
    type Reader = <Listener::Stream as AsyncDataStream>::ReadHalf;
    type Writer = <Listener::Stream as AsyncDataStream>::WriteHalf;
    type Error = Error<Listener::Error, <Listener::Stream as AsyncDataStream>::Error>;

    async fn connect(
        &self,
        locator: &NodeLocator,
    ) -> Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error> {
        if let Some(session) = self.manager.read().await.get(&locator.designator) {
            return Ok(session.clone());
        }

        tracing::debug!(
            local = ?self.locators(),
            remote = %syrup::ser::to_pretty(locator).unwrap(),
            "starting connection"
        );

        let (reader, writer) = <Listener::Stream as AsyncDataStream>::connect(locator)
            .await
            .map_err(Error::Stream)?
            .split();

        self.manager
            .write()
            .await
            .init_session(reader, writer)
            .and_connect(self.locators().pop().unwrap())
            .await
            .map_err(From::from)
    }

    async fn accept(&self) -> Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error> {
        tracing::debug!(
            local = ?self.locators(),
            "accepting connection"
        );

        let (reader, writer) =
            futures::future::select_all(self.listeners.iter().map(|listener| listener.accept()))
                .await
                .0
                .map_err(Error::Listener)?
                .0
                .split();

        self.manager
            .write()
            .await
            .init_session(reader, writer)
            .and_accept(self.locators().pop().unwrap())
            .await
            .map_err(From::from)
    }

    fn locators(&self) -> Vec<NodeLocator> {
        self.listeners
            .iter()
            .map(|l| l.locator().unwrap())
            .collect()
    }
}

impl<Listener: AsyncStreamListener> DataStreamNetlayer<Listener> {
    pub fn new(listeners: Vec<Listener>) -> Self {
        Self {
            listeners,
            manager: RwLock::new(CapTpSessionManager::new()),
        }
    }

    pub async fn bind(addr: Listener::AddressInput<'_>) -> Result<Self, Listener::Error> {
        let listener = Listener::bind(addr).await?;
        Ok(Self::new(vec![listener]))
    }

    pub async fn push_bind(
        &mut self,
        addr: Listener::AddressInput<'_>,
    ) -> Result<(), Listener::Error> {
        self.listeners.push(Listener::bind(addr).await?);
        Ok(())
    }
}
