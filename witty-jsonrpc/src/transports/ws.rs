use std::sync::{Arc, Mutex};

use jsonrpc_ws_server::{
    jsonrpc_core::{MetaIoHandler, Metadata, NoopMiddleware},
    Server, ServerBuilder,
};
pub use jsonrpc_ws_server::Error;

use crate::{Transport, transports::TransportError};

pub struct WsTransportSettings {
    pub address: String,
}

pub struct WsTransport<M> where M: Metadata {
    settings: WsTransportSettings,
    server_builder: Option<ServerBuilder<M, NoopMiddleware>>,
    server: Option<Server>,
}

impl<M> WsTransport<M> where M: Metadata {
    pub fn new(settings: WsTransportSettings) -> Self {
        Self {
            settings,
            server_builder: None,
            server: None,
        }
    }
}

impl<M> Transport<M> for WsTransport<M> where M: Metadata + Default {
    fn requires_reset(&self) -> bool {
        true
    }

    fn running(&self) -> bool {
        self.server.is_some()
    }

    fn set_handler(&mut self, handler: Arc<Mutex<MetaIoHandler<M>>>) -> Result<(), TransportError> {
        let handler = (*handler.lock().unwrap()).clone();

        self.server_builder = Some(ServerBuilder::new(handler));

        Ok(())
    }

    fn start(&mut self, _meta: M) -> Result<(), TransportError> {
        if self.running() {
            return Ok(());
        }

        let builder = self
            .server_builder
            .take()
            .ok_or(TransportError::NoHandler)?;
        let socket_addr = self.settings.address.parse::<std::net::SocketAddr>()?;
        self.server = Some(builder.start(&socket_addr)?);

        Ok(())
    }

    fn stop(&mut self) -> Result<(), TransportError> {
        match self.server.take() {
            None => Ok(()),
            Some(server) => {
                server.close();

                Ok(())
            },
        }
    }
}
