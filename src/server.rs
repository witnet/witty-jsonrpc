use std::sync::{Arc, Mutex};

#[cfg(feature = "with_actix")]
use actix::System;
use jsonrpc_core::{Metadata, RpcMethodSimple};
use jsonrpc_pubsub::{PubSubHandler, SubscribeRpcMethod, UnsubscribeRpcMethod};

use crate::{
    handler::{Handler, Session},
    transports::{Transport, TransportError},
};

/// A convenient type alias for a single transport server that supports PubSub.
pub type WittyMonoServer = SingleTransportServer<PubSubHandler<Session>>;
/// A convenient type alias for a multiple transports server that supports PubSub.
pub type WittyMultiServer = MultipleTransportsServer<PubSubHandler<Session>>;

/// Enumerates all the different errors that a `Server` can get into.
#[derive(Debug)]
pub enum ServerError {
    /// An error that happened in one of the underlaying transports
    Transport(TransportError),
}

impl From<TransportError> for ServerError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

/// Trait defining a JSON-RPC server.
pub trait Server<H>
where
    H: Handler,
{
    /// The type to use as the error type within the `Result`s used by an implementation of this
    /// trait.
    type Error;

    /// Start the server.
    ///
    /// This is expexted to be highly side effected, i.e. this is where sockets and listeners are
    /// started.
    fn start(&mut self) -> Result<(), Self::Error>;

    /// Stop the server.
    fn stop(&mut self) -> Result<(), Self::Error>;

    /// Add a JSON-RPC method to the server.
    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple;

    /// Add a JSON-RPC subscription so the server.
    fn add_subscription<F, G>(
        &mut self,
        notification: &str,
        subscribe: (&str, F),
        unsubscribe: (&str, G),
    ) where
        F: SubscribeRpcMethod<H::Metadata>,
        G: UnsubscribeRpcMethod<H::Metadata>;

    /// Get a list of all the supported JSON-RPC methods.
    fn describe_api(&self) -> Vec<String>;
}

/// A little extension of `Server` that allows seamless compatibility with the Actix framework.
///
/// This trait is only conditionally compiled. To use it, make sure to enable the `with_actix`
/// feature in `Cargo.toml`.
#[cfg(feature = "with_actix")]
pub trait ActixServer<H>: Server<H>
where
    H: Handler,
{
    /// Add a JSON-RPC method that when executed will be spawned into an Actix `arbiter` if a
    /// `system` is provided.
    fn add_actix_method<F>(&mut self, system: &Option<actix::System>, name: &str, method: F)
    where
        F: RpcMethodSimple;

    /// Add a JSON-RPC subscription that when executed will be spawned into an Actix `arbiter` if a
    /// `system` is provided.
    fn add_actix_subscription<F, G>(
        &mut self,
        system: &Option<actix::System>,
        notification: &str,
        subscribe: (&str, Arc<F>),
        unsubscribe: (&str, Arc<G>),
    ) where
        F: SubscribeRpcMethod<H::Metadata>,
        G: UnsubscribeRpcMethod<
            H::Metadata,
            Out = jsonrpc_core::BoxFuture<jsonrpc_core::Result<jsonrpc_core::Value>>,
        >;
}

/// A JSON-RPC server that supports using multiple transports at once.
///
/// All the transports share the same underlying IO handler.
#[derive(Default)]
pub struct MultipleTransportsServer<H>
where
    H: Handler,
{
    transports: Vec<Box<dyn Transport<H>>>,
    // TODO: Change Mutex for RwLock
    io_handler: Arc<Mutex<H>>,
}

impl<H> MultipleTransportsServer<H>
where
    H: Handler,
{
    /// Add a transport to the server.
    pub fn add_transport<T>(&mut self, mut transport: T)
    where
        T: Transport<H> + 'static,
    {
        transport.set_handler(self.io_handler.clone()).ok();
        self.transports.push(Box::new(transport));
    }

    /// Programmatically trigger the handling of a JSON-RPC message inside the IO handler that the
    /// server wraps.
    pub fn handle_request_sync(&self, request: &str, meta: H::Metadata) -> Option<String> {
        self.io_handler
            .lock()
            .unwrap()
            .handle_request_sync(request, meta)
    }

    /// Apply the same closure on every single transport added to this server.
    fn on_every_transport<'a, F, O>(&mut self, mut operation: F) -> Result<Vec<O>, TransportError>
    where
        F: FnMut(&mut (dyn Transport<H> + 'a)) -> Result<O, TransportError>,
    {
        self.transports
            .iter_mut()
            .map(|transport| operation(&mut **transport))
            .collect::<Result<Vec<_>, _>>()
    }

    /// Create a new server with everything set to its defaults.
    pub fn new() -> Self {
        Self {
            transports: vec![],
            io_handler: Arc::new(Mutex::new(H::new())),
        }
    }

    /// Stop, reconfigure and re-start all the transports added to this server.
    ///
    /// This is especially needed for transports that use a external server builder and therefore
    /// cannot benefit from the `Arc` around the IO handler.
    fn reset_all_transports(&mut self) -> Result<(), TransportError> {
        let handler = self.io_handler.clone();

        self.on_every_transport(|transport| {
            if transport.requires_reset() {
                let running = transport.running();
                if running {
                    transport.stop()?;
                }
                transport.set_handler(handler.clone())?;
                if running {
                    transport.start()?;
                }
            }
            Ok(())
        })?;

        Ok(())
    }
}

impl<H> Server<H> for MultipleTransportsServer<H>
where
    H: Handler,
    H::Metadata: Metadata,
{
    type Error = ServerError;

    fn start(&mut self) -> Result<(), Self::Error> {
        let _ = &self.on_every_transport(|transport| transport.start())?;

        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        let _ = &self.on_every_transport(Transport::stop)?;

        Ok(())
    }

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple,
    {
        (*self.io_handler.lock().unwrap()).add_method(name, method);
        self.reset_all_transports().ok();
    }

    fn add_subscription<F, G>(
        &mut self,
        notification: &str,
        subscribe: (&str, F),
        unsubscribe: (&str, G),
    ) where
        F: SubscribeRpcMethod<H::Metadata>,
        G: UnsubscribeRpcMethod<H::Metadata>,
    {
        (*self.io_handler.lock().unwrap()).add_subscription(notification, subscribe, unsubscribe);
        self.reset_all_transports().ok();
    }

    fn describe_api(&self) -> Vec<String> {
        self.io_handler.lock().unwrap().describe_api()
    }
}

#[cfg(feature = "with_actix")]
impl<H> ActixServer<H> for MultipleTransportsServer<H>
where
    H: Handler,
{
    fn add_actix_method<F>(&mut self, system: &Option<actix::System>, name: &str, method: F)
    where
        F: RpcMethodSimple,
    {
        let system = system.clone();

        self.add_method(name, move |params| {
            let system = system.clone();
            let execution = method.call(params);
            let (tx, rx) = futures::channel::oneshot::channel();

            Box::pin(async move {
                // The future that will actually execute the method
                let fut = async move {
                    let response = execution.await;
                    tx.send(response)
                        .expect("Should be able to send result back to spawner");
                };

                // If an actix system is available, spawn there, otherwise simply wait on the future
                if let Some(system) = system.clone() {
                    system.arbiter().spawn(fut);
                } else {
                    fut.await;
                }

                rx.await
                    .expect("Should be able to await the oneshot channel")
            })
        })
    }

    fn add_actix_subscription<F, G>(
        &mut self,
        system: &Option<System>,
        notification: &str,
        subscribe: (&str, Arc<F>),
        unsubscribe: (&str, Arc<G>),
    ) where
        F: SubscribeRpcMethod<H::Metadata>,
        G: UnsubscribeRpcMethod<
            H::Metadata,
            Out = jsonrpc_core::BoxFuture<jsonrpc_core::Result<jsonrpc_core::Value>>,
        >,
    {
        let subscribe_system = system.clone();
        let unsubscribe_system = system.clone();
        let (subscribe_name, subscribe_method) = subscribe;
        let (unsubscribe_name, unsubscribe_method) = unsubscribe;

        self.add_subscription(
            notification,
            (subscribe_name, move |params, meta, subscriber| {
                let method = subscribe_method.clone();

                // If an actix system is available, spawn there, otherwise simply wait on the future
                if let Some(system) = subscribe_system.clone() {
                    system.arbiter().spawn(async move {
                        method.call(params, meta, subscriber);
                    });
                } else {
                    method.call(params, meta, subscriber);
                }
            }),
            (unsubscribe_name, move |id, meta| {
                let system = unsubscribe_system.clone();
                let method = unsubscribe_method.clone();
                let execution = method.call(id, meta);
                let (tx, rx) = futures::channel::oneshot::channel();

                Box::pin(async move {
                    // The future that will actually execute the method
                    let fut = async move {
                        let response = execution.await;
                        tx.send(response)
                            .expect("Should be able to send result back to spawner");
                    };

                    // If an actix system is available, spawn there, otherwise simply wait on the future
                    if let Some(system) = system.clone() {
                        system.arbiter().spawn(fut);
                    } else {
                        fut.await;
                    }

                    rx.await
                        .expect("Should be able to await the oneshot channel")
                })
            }),
        )
    }
}

/// A simple JSON-RPC server that only uses one transport.
pub struct SingleTransportServer<H>
where
    H: Handler,
{
    inner: MultipleTransportsServer<H>,
}

impl<H> SingleTransportServer<H>
where
    H: Handler,
{
    /// Create a simple server around an already existing instance of a transport.
    pub fn from_transport<T>(transport: T) -> Self
    where
        T: Transport<H> + 'static,
    {
        let mut inner = MultipleTransportsServer::new();
        inner.add_transport(transport);

        Self { inner }
    }

    /// Programmatically trigger the handling of a JSON-RPC message on this server.
    pub fn handle_request_sync(&self, request: &str, meta: H::Metadata) -> Option<String> {
        self.inner.handle_request_sync(request, meta)
    }

    /// Create a new server.
    pub fn new<T>() -> Self
    where
        T: Transport<H> + 'static,
    {
        let inner = MultipleTransportsServer::new();

        Self { inner }
    }
}

impl<H> Server<H> for SingleTransportServer<H>
where
    H: Handler,
{
    type Error = ServerError;

    fn start(&mut self) -> Result<(), Self::Error> {
        Server::start(&mut self.inner)
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        Server::stop(&mut self.inner)
    }

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple,
    {
        Server::add_method(&mut self.inner, name, method)
    }

    fn add_subscription<F, G>(
        &mut self,
        notification: &str,
        subscribe: (&str, F),
        unsubscribe: (&str, G),
    ) where
        F: SubscribeRpcMethod<H::Metadata>,
        G: UnsubscribeRpcMethod<H::Metadata>,
    {
        Server::add_subscription(&mut self.inner, notification, subscribe, unsubscribe)
    }

    fn describe_api(&self) -> Vec<String> {
        Server::describe_api(&self.inner)
    }
}

#[cfg(feature = "with_actix")]
impl<H> ActixServer<H> for SingleTransportServer<H>
where
    H: Handler,
{
    fn add_actix_method<F>(&mut self, system: &Option<System>, name: &str, method: F)
    where
        F: RpcMethodSimple,
    {
        ActixServer::add_actix_method(&mut self.inner, system, name, method)
    }

    fn add_actix_subscription<F, G>(
        &mut self,
        system: &Option<System>,
        notification: &str,
        subscribe: (&str, Arc<F>),
        unsubscribe: (&str, Arc<G>),
    ) where
        F: SubscribeRpcMethod<H::Metadata>,
        G: UnsubscribeRpcMethod<
            H::Metadata,
            Out = jsonrpc_core::BoxFuture<jsonrpc_core::Result<jsonrpc_core::Value>>,
        >,
    {
        ActixServer::add_actix_subscription::<F, G>(
            &mut self.inner,
            system,
            notification,
            subscribe,
            unsubscribe,
        )
    }
}
