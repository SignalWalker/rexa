use std::{
    borrow::Cow,
    collections::HashMap,
    future::Future,
    sync::{Arc, Weak},
};

use parking_lot::RwLock;
use rexa::{
    captp::{CapTpSessionManager, SessionInitError},
    locator::NodeLocator,
    netlayer::Netlayer,
};

use tokio::{
    io::{BufReader, DuplexStream},
    sync::{mpsc, oneshot, Mutex as AsyncMutex, RwLock as AsyncRwLock},
};

type MockReader = <MockNetlayer as Netlayer>::Reader;
type MockWriter = <MockNetlayer as Netlayer>::Writer;
type StreamSend = oneshot::Sender<(MockReader, MockWriter)>;

type MockRegistry =
    RwLock<HashMap<String, (Weak<MockNetlayer>, mpsc::UnboundedSender<StreamSend>)>>;

lazy_static::lazy_static! {
    static ref MOCK_REGISTRY: MockRegistry = RwLock::default();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("name already in use")]
    NameInUse,
    #[error("address not found")]
    NotFound,
    #[error("address found in registry, but the receiver has been dropped")]
    ReceiverDropped,
    #[error("MockNetlayer registry poisoned")]
    RegistryPoisoned,
    #[error("pipe broken during accept")]
    Accept,
    #[error(transparent)]
    Connect(#[from] oneshot::error::RecvError),
    #[error(transparent)]
    Init(#[from] SessionInitError),
}

impl<Guard> From<std::sync::PoisonError<Guard>> for Error {
    fn from(_: std::sync::PoisonError<Guard>) -> Self {
        Self::RegistryPoisoned
    }
}

pub struct MockNetlayer {
    name: String,
    connect_recv: AsyncMutex<mpsc::UnboundedReceiver<StreamSend>>,
    manager: AsyncRwLock<CapTpSessionManager<MockReader, MockWriter>>,
}

impl MockNetlayer {
    pub fn bind(name: String) -> Result<Arc<Self>, Error> {
        let mut reg = MOCK_REGISTRY.write();
        if let Some(res) = reg.get(&name).and_then(|(p, _)| Weak::upgrade(p)) {
            Ok(res)
        } else {
            let (connect_send, connect_recv) = mpsc::unbounded_channel();
            let res = Arc::new(Self {
                name: name.clone(),
                connect_recv: AsyncMutex::new(connect_recv),
                manager: AsyncRwLock::new(CapTpSessionManager::new()),
            });
            reg.insert(name, (Arc::downgrade(&res), connect_send));
            Ok(res)
        }
    }

    pub fn close(self) {
        MOCK_REGISTRY.write().remove(&self.name);
    }
}

impl Netlayer for MockNetlayer {
    type Reader = BufReader<DuplexStream>;
    type Writer = DuplexStream;
    type Error = Error;

    fn connect<'locator>(
        &self,
        locator: &rexa::locator::NodeLocator<'locator>,
    ) -> impl Future<
        Output = Result<rexa::captp::CapTpSession<Self::Reader, Self::Writer>, Self::Error>,
    > + Send {
        let remote_name = &locator.designator;
        async move {
            if let Some(session) = self.manager.read().await.get(remote_name) {
                return Ok(session.clone());
            }

            let (stream_send, stream_recv) = oneshot::channel();
            if MOCK_REGISTRY
                .read()
                .get(&*locator.designator)
                .ok_or(Error::NotFound)?
                .1
                .send(stream_send)
                .is_err()
            {
                // send failed, therefore receiver has been dropped; clean registry
                MOCK_REGISTRY.write().remove(&*locator.designator);
                return Err(Error::NotFound);
            }

            let (reader, writer) = stream_recv.await?;
            self.manager
                .write()
                .await
                .init_session(reader, writer)
                .and_connect(NodeLocator::new(&self.name, "mock"))
                .await
                .map_err(From::from)
        }
    }

    async fn accept(
        &self,
    ) -> Result<rexa::captp::CapTpSession<Self::Reader, Self::Writer>, Self::Error> {
        let stream_send = self.connect_recv.lock().await.recv().await.unwrap();
        let (reader, writer) = {
            // HACK :: there's probably a better way to set this number but whatever
            let (local_reader, remote_writer) = tokio::io::duplex(1024);
            let (remote_reader, local_writer) = tokio::io::duplex(1024);
            stream_send
                .send((BufReader::new(remote_reader), remote_writer))
                .map_err(|_err| Error::Accept)?;
            (local_reader, local_writer)
        };
        self.manager
            .write()
            .await
            .init_session(BufReader::new(reader), writer)
            .and_connect(NodeLocator::new(&self.name, "mock"))
            .await
            .map_err(From::from)
    }

    fn locators(&self) -> Vec<rexa::locator::NodeLocator<'_>> {
        vec![NodeLocator::new(
            Cow::Borrowed(self.name.as_str()),
            Cow::Borrowed("mock"),
        )]
    }
}
