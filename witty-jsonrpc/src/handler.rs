use std::{fmt::Debug, sync::Arc};

use futures::channel::mpsc::UnboundedSender;
use jsonrpc_core::{MetaIoHandler, Metadata, RpcMethodSimple};
use jsonrpc_pubsub::{PubSubHandler, PubSubMetadata, SubscribeRpcMethod, UnsubscribeRpcMethod};

#[derive(Clone, Debug, Default)]
pub struct Session {
    inner: Option<Arc<jsonrpc_pubsub::Session>>,
}

impl From<Arc<jsonrpc_pubsub::Session>> for Session {
    fn from(value: Arc<jsonrpc_pubsub::Session>) -> Self {
        Session { inner: Some(value) }
    }
}

impl Metadata for Session {}

impl PubSubMetadata for Session {
    fn session(&self) -> Option<Arc<jsonrpc_pubsub::Session>> {
        self.inner.clone()
    }
}

pub trait Handler {
    type Metadata: PubSubMetadata + Unpin + Debug;

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple;

    fn add_subscription<F, G>(
        &mut self,
        notification: &str,
        subscribe: (&str, F),
        unsubscribe: (&str, G),
    ) where
        F: SubscribeRpcMethod<Self::Metadata>,
        G: UnsubscribeRpcMethod<Self::Metadata>;

    fn as_meta_io_handler(&self) -> MetaIoHandler<Self::Metadata>;

    fn describe_api(&self) -> Vec<String>;

    // TODO: support async
    fn handle_request_sync(&self, request: &str, meta: Self::Metadata) -> Option<String>;

    fn metadata_from_sender(sender: UnboundedSender<String>) -> Self::Metadata;

    fn new() -> Self;
}

impl<M> Handler for PubSubHandler<M>
where
    M: PubSubMetadata + Unpin + Debug + From<Arc<jsonrpc_pubsub::Session>>,
{
    type Metadata = M;

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple,
    {
        MetaIoHandler::add_method(self, name, method)
    }

    fn add_subscription<F, G>(
        &mut self,
        notification: &str,
        subscribe: (&str, F),
        unsubscribe: (&str, G),
    ) where
        F: SubscribeRpcMethod<M>,
        G: UnsubscribeRpcMethod<M>,
    {
        PubSubHandler::add_subscription(self, notification, subscribe, unsubscribe)
    }

    fn as_meta_io_handler(&self) -> MetaIoHandler<Self::Metadata> {
        (*self).clone()
    }

    fn describe_api(&self) -> Vec<String> {
        self.iter().map(|(name, _)| name.clone()).collect()
    }

    fn handle_request_sync(&self, request: &str, meta: Self::Metadata) -> Option<String> {
        MetaIoHandler::handle_request_sync(self, request, meta)
    }

    fn metadata_from_sender(sender: UnboundedSender<String>) -> Self::Metadata {
        Self::Metadata::from(Arc::new(jsonrpc_pubsub::Session::new(sender)))
    }

    fn new() -> Self {
        PubSubHandler::new(jsonrpc_core::MetaIoHandler::default())
    }
}
