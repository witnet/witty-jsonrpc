use jsonrpc_core::{RpcMethodSimple, MetaIoHandler, Metadata};

use crate::{transports::TransportError, Transport};
use std::sync::{Arc, Mutex};

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

pub trait Server {
    type Error;

    fn start(&mut self) -> Result<(), Self::Error>;
    fn stop(&mut self) -> Result<(), Self::Error>;

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple;
}

#[cfg(feature = "with_actix")]
pub trait ActixServer: Server {
    fn add_actix_method<F>(&mut self, system: &actix::System, name: &str, method: F) where F: RpcMethodSimple;
}

#[derive(Default)]
pub struct MultipleTransportsServer<M> where M: Metadata {
    transports: Vec<Box<dyn Transport<M>>>,
    // TODO: Change Mutex for RwLock
    io_handler: Arc<Mutex<MetaIoHandler<M>>>,
    meta: M,
}

impl<M> MultipleTransportsServer<M> where M: Metadata {
    pub fn add_transport<T>(&mut self, mut transport: T)
    where
        T: Transport<M> + 'static,
    {
        transport.set_handler(self.io_handler.clone()).ok();
        self.transports.push(Box::new(transport));
    }

    pub fn from_handler(handler: MetaIoHandler<M>, meta: M) -> Self {
        let mut server = Self::new(meta);
        server.io_handler = Arc::new(Mutex::new(handler));

        server
    }

    fn on_every_transport<'a, F, O>(&mut self, mut operation: F) -> Result<Vec<O>, TransportError>
    where
        F: FnMut(&mut (dyn Transport<M> + 'a)) -> Result<O, TransportError>,
    {
        self.transports
            .iter_mut()
            .map(|transport| operation(&mut **transport))
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn new(meta: M) -> Self {
        Self {
            transports: vec![],
            io_handler: Arc::new(Mutex::new(MetaIoHandler::default())),
            meta
        }
    }

    pub fn reset_all_transports(&mut self) -> Result<(), TransportError> {
        let handler = self.io_handler.clone();
        let meta = self.meta.clone();

        self.on_every_transport(|transport| {
            if transport.requires_reset() {
                let running = transport.running();
                if running {
                    transport.stop()?;
                }
                transport.set_handler(handler.clone())?;
                if running {
                    transport.start(meta.clone())?;
                }
            }
            Ok(())
        })?;

        Ok(())
    }
}

impl<M> Server for MultipleTransportsServer<M> where M: Metadata {
    type Error = ServerError;

    fn start(&mut self) -> Result<(), Self::Error> {
        let meta = self.meta.clone();
        let _ = &self.on_every_transport(|transport| transport.start(meta.clone()))?;

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
}

#[cfg(feature = "with_actix")]
impl<M> ActixServer for MultipleTransportsServer<M> where M: Metadata {
    fn add_actix_method<F>(&mut self, system: &actix::System, name: &str, method: F) where F: RpcMethodSimple {
        let system = system.clone();

        self.add_method(name, move |params| {
            let (tx, rx) = futures::channel::oneshot::channel();

            let execution = method.call(params);

            system
                .arbiter()
                .spawn(async move {
                    let response = execution.await;
                    tx.send(response).unwrap();
                });

            Box::pin(async {
                rx.await.unwrap()
            })

        })
    }
}

pub struct SingleTransportServer<M = ()> where M: Metadata {
    inner: MultipleTransportsServer<M>,
}

impl<M> SingleTransportServer<M> where M: Metadata {
    pub fn from_transport<T>(transport: T, meta: M) -> Self
    where
        T: Transport<M> + 'static,
    {
        let mut inner = MultipleTransportsServer::new(meta);
        inner.add_transport(transport);

        Self { inner }
    }
}

impl<M> Server for SingleTransportServer<M> where M: Metadata {
    type Error = ServerError;

    fn start(&mut self) -> Result<(), Self::Error> {
        self.inner.start()
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        self.inner.stop()
    }

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSimple,
    {
        self.inner.add_method(name, method)
    }
}
