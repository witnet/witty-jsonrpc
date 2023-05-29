use jsonrpc_core::{IoHandler, RpcMethodSync};

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
        F: RpcMethodSync;
}

#[derive(Default)]
pub struct MultipleTransportsServer {
    transports: Vec<Box<dyn Transport>>,
    io_handler: Arc<Mutex<IoHandler>>,
}

impl MultipleTransportsServer {
    pub fn add_transport<T>(&mut self, mut transport: T)
    where
        T: Transport + 'static,
    {
        transport.set_handler(self.io_handler.clone()).ok();
        self.transports.push(Box::new(transport));
    }

    fn on_every_transport<'a, F, O>(&mut self, mut operation: F) -> Result<Vec<O>, TransportError>
    where
        F: FnMut(&mut (dyn Transport + 'a)) -> Result<O, TransportError>,
    {
        self.transports
            .iter_mut()
            .map(|transport| operation(&mut **transport))
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn new() -> Self {
        Self::default()
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

impl Server for MultipleTransportsServer {
    type Error = ServerError;

    fn start(&mut self) -> Result<(), Self::Error> {
        let _ = &self.on_every_transport(Transport::start)?;

        Ok(())
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        let _ = &self.on_every_transport(Transport::stop)?;

        Ok(())
    }

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSync,
    {
        (*self.io_handler.lock().unwrap()).add_sync_method(name, method);
        self.reset_all_transports().ok();
    }
}

pub struct SingleTransportServer {
    inner: MultipleTransportsServer,
}

impl SingleTransportServer {
    pub fn from_transport<T>(transport: T) -> Self
    where
        T: Transport + 'static,
    {
        let mut inner = MultipleTransportsServer::default();
        inner.add_transport(transport);

        Self { inner }
    }
}

impl Server for SingleTransportServer {
    type Error = ServerError;

    fn start(&mut self) -> Result<(), Self::Error> {
        self.inner.start()
    }

    fn stop(&mut self) -> Result<(), Self::Error> {
        self.inner.stop()
    }

    fn add_method<F>(&mut self, name: &str, method: F)
    where
        F: RpcMethodSync,
    {
        self.inner.add_method(name, method)
    }
}
