use std::sync::Arc;

use arti_client::{DataReader, DataWriter, TorClient, TorClientConfig};
use futures::{lock::Mutex, stream::BoxStream, StreamExt};
use rexa::{
    captp::{CapTpSession, CapTpSessionManager, SessionInitError},
    locator::NodeLocator,
    netlayer::Netlayer,
};
use tor_cell::relaycell::msg::Connected;
use tor_hsservice::{OnionServiceConfig, RunningOnionService, StreamRequest};
use tor_rtcompat::Runtime;
// TODO :: remove hard tokio dependency from rexa-netlayer-onion
use tokio::{io::BufReader, sync::RwLock};

#[repr(transparent)]
struct TorLocator<'l>(&'l NodeLocator<'l>);

impl<'l> From<&'l NodeLocator<'l>> for TorLocator<'l> {
    fn from(value: &'l NodeLocator<'l>) -> Self {
        Self(value)
    }
}

impl<'l> std::ops::Deref for TorLocator<'l> {
    type Target = NodeLocator<'l>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'l> arti_client::IntoTorAddr for TorLocator<'l> {
    fn into_tor_addr(self) -> Result<arti_client::TorAddr, arti_client::TorAddrError> {
        format!("{}.onion", self.designator).into_tor_addr()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    //#[error(transparent)]
    //Io(#[from] std::io::Error),
    #[error(transparent)]
    Tor(#[from] arti_client::Error),
    #[error(transparent)]
    Client(#[from] tor_hsservice::ClientError),
    #[error("session manager lock poisoned")]
    LockPoisoned,
    #[error(transparent)]
    Init(#[from] SessionInitError),
}

impl<Guard> From<std::sync::PoisonError<Guard>> for Error {
    fn from(_: std::sync::PoisonError<Guard>) -> Self {
        Self::LockPoisoned
    }
}

pub struct OnionNetlayer<AsyncRuntime: Runtime> {
    service: Arc<RunningOnionService>,
    req_stream: Mutex<BoxStream<'static, StreamRequest>>,
    client: TorClient<AsyncRuntime>,
    manager: RwLock<CapTpSessionManager<<Self as Netlayer>::Reader, <Self as Netlayer>::Writer>>,
}

impl<Rt: Runtime> std::fmt::Debug for OnionNetlayer<Rt> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnionNetlayer")
            .field("locator", &self.locators().pop().unwrap())
            .finish_non_exhaustive()
    }
}

impl<Rt: Runtime> OnionNetlayer<Rt> {
    pub fn new(
        client: TorClient<Rt>,
        service_config: OnionServiceConfig,
    ) -> Result<Self, arti_client::Error> {
        let (service, stream) = client.launch_onion_service(service_config)?;
        Ok(Self {
            service,
            req_stream: tor_hsservice::handle_rend_requests(stream).boxed().into(),
            client,
            manager: RwLock::new(CapTpSessionManager::new()),
        })
    }

    pub fn service(&self) -> &RunningOnionService {
        &self.service
    }

    pub async fn new_bootstrapped(
        runtime: Rt,
        client_config: TorClientConfig,
        service_config: OnionServiceConfig,
    ) -> Result<Self, arti_client::Error> {
        let client = TorClient::with_runtime(runtime)
            .config(client_config)
            .create_bootstrapped()
            .await?;
        Self::new(client, service_config)
    }

    pub fn designator(&self) -> String {
        // HACK :: there's probably a better way to do this
        let mut name = self
            .service
            .onion_name()
            .expect("OnionNetlayer should know own onion service name")
            .to_string();
        name.truncate(name.len() - ".onion".len());
        name
    }
}

impl<R: Runtime> Netlayer for OnionNetlayer<R> {
    type Reader = BufReader<DataReader>;
    type Writer = DataWriter;
    type Error = Error;

    #[inline]
    async fn connect<'loc>(
        &self,
        locator: &NodeLocator<'loc>,
    ) -> Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error> {
        let (reader, writer) = self.client.connect(TorLocator(locator)).await?.split();
        self.manager
            .write()
            .await
            .init_session(BufReader::new(reader), writer)
            .and_connect(NodeLocator::new(self.designator(), "onion"))
            .await
            .map_err(From::from)
    }

    async fn accept(&self) -> Result<CapTpSession<Self::Reader, Self::Writer>, Self::Error> {
        let (reader, writer) = self
            .req_stream
            .lock()
            .await
            .next()
            .await
            .expect("req_stream should always return Some(..)")
            .accept(Connected::new_empty())
            .await?
            .split();

        self.manager
            .write()
            .await
            .init_session(BufReader::new(reader), writer)
            .and_accept(NodeLocator::new(self.designator(), "onion"))
            .await
            .map_err(From::from)
    }

    fn locators(&self) -> Vec<NodeLocator<'_>> {
        vec![NodeLocator::new(self.designator(), "onion")]
    }
}
