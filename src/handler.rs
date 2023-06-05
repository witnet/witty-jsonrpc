use std::{fmt::Debug, sync::Arc};

use futures::channel::mpsc::UnboundedSender;
use jsonrpc_core::{MetaIoHandler, Metadata, RpcMethodSimple};
use jsonrpc_pubsub::{PubSubHandler, PubSubMetadata, SubscribeRpcMethod, UnsubscribeRpcMethod};

/// A wrapper around `jsonrpc_core`'s own `Session`, providing some convenience methods and
/// `impl Default::default`.
#[derive(Clone, Debug, Default)]
pub struct Session {
    inner: Option<Arc<jsonrpc_pubsub::Session>>,
}

impl Session {
    /// A mocked session, i.e. created from an mpsc channel that is not wired to anything.
    pub fn mock() -> Session {
        let (sender, _) = futures::channel::mpsc::unbounded();
        let inner = Some(Arc::from(jsonrpc_pubsub::Session::new(sender)));

        Self { inner }
    }
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

/// Trait that abstracts away different implementations of IO handlers.
pub trait Handler {
    /// The type to use as the metadata for the IO handler. Especially relevant for PubSub.
    type Metadata: PubSubMetadata + Unpin + Debug;

    /// Add a JSON-RPC method.
    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple;

    /// Add a JSON-RPC subscription.
    fn add_subscription<F, G>(
        &mut self,
        notification: &str,
        subscribe: (&str, F),
        unsubscribe: (&str, G),
    ) where
        F: SubscribeRpcMethod<Self::Metadata>,
        G: UnsubscribeRpcMethod<Self::Metadata>;

    /// Cast this down to an instance of `MetaIoHandler`.
    fn as_meta_io_handler(&self) -> MetaIoHandler<Self::Metadata>;

    /// Get a list of all the supported JSON-RPC methods.
    fn describe_api(&self) -> Vec<String>;

    /// Programmatically trigger the handling of a JSON-RPC message.
    /// TODO: support async
    fn handle_request_sync(&self, request: &str, meta: Self::Metadata) -> Option<String>;

    /// Derive an instance of the `Self::Metadata` associated type from an `UnboundedSender`.
    fn metadata_from_sender(sender: UnboundedSender<String>) -> Self::Metadata;

    /// Create a new instance of a `Handler` implementation.
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
