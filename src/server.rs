use std::sync::{Arc, Mutex};

#[cfg(feature = "with_actix")]
use actix::System;
use jsonrpc_core::{Metadata, RpcMethodSimple};
use jsonrpc_pubsub::{PubSubHandler, SubscribeRpcMethod, UnsubscribeRpcMethod};

use crate::{
    handler::{Handler, Session},
    transports::{Transport, TransportError},
};

pub type WittyMonoServer = SingleTransportServer<PubSubHandler<Session>>;
pub type WittyMultiServer = MultipleTransportsServer<PubSubHandler<Session>>;

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

pub trait Server<H>
where
    H: Handler,
{
    type Error;

    fn start(&mut self) -> Result<(), Self::Error>;
    fn stop(&mut self) -> Result<(), Self::Error>;

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple;

    fn add_subscription<F, G>(
        &mut self,
        notification: &str,
        subscribe: (&str, F),
        unsubscribe: (&str, G),
    ) where
        F: SubscribeRpcMethod<H::Metadata>,
        G: UnsubscribeRpcMethod<H::Metadata>;

    fn describe_api(&self) -> Vec<String>;
}

#[cfg(feature = "with_actix")]
pub trait ActixServer<H>: Server<H>
where
    H: Handler,
{
    fn add_actix_method<F>(&mut self, system: &Option<actix::System>, name: &str, method: F)
    where
        F: RpcMethodSimple;

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
    pub fn add_transport<T>(&mut self, mut transport: T)
    where
        T: Transport<H> + 'static,
    {
        transport.set_handler(self.io_handler.clone()).ok();
        self.transports.push(Box::new(transport));
    }

    pub fn handle_request_sync(&self, request: &str, meta: H::Metadata) -> Option<String> {
        self.io_handler
            .lock()
            .unwrap()
            .handle_request_sync(request, meta)
    }

    fn on_every_transport<'a, F, O>(&mut self, mut operation: F) -> Result<Vec<O>, TransportError>
    where
        F: FnMut(&mut (dyn Transport<H> + 'a)) -> Result<O, TransportError>,
    {
        self.transports
            .iter_mut()
            .map(|transport| operation(&mut **transport))
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn new() -> Self {
        Self {
            transports: vec![],
            io_handler: Arc::new(Mutex::new(H::new())),
        }
    }

    pub fn reset_all_transports(&mut self) -> Result<(), TransportError> {
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
    pub fn from_transport<T>(transport: T) -> Self
    where
        T: Transport<H> + 'static,
    {
        let mut inner = MultipleTransportsServer::new();
        inner.add_transport(transport);

        Self { inner }
    }

    pub fn handle_request_sync(&self, request: &str, meta: H::Metadata) -> Option<String> {
        self.inner.handle_request_sync(request, meta)
    }

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
