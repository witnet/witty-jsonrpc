use std::sync::{Arc, Mutex};

use jsonrpc_ws_server::{
    jsonrpc_core::{IoHandler, NoopMiddleware},
    Server, ServerBuilder,
};

pub use jsonrpc_ws_server::Error;

use crate::{transports::TransportError, Transport};

pub struct WsTransportSettings {
    pub address: String,
}

pub struct WsTransport {
    settings: WsTransportSettings,
    server_builder: Option<ServerBuilder<(), NoopMiddleware>>,
    server: Option<Server>,
}

impl WsTransport {
    pub fn new(settings: WsTransportSettings) -> Self {
        Self {
            settings,
            server_builder: None,
            server: None,
        }
    }
}

impl Transport for WsTransport {
    fn requires_reset(&self) -> bool {
        true
    }

    fn running(&self) -> bool {
        self.server.is_some()
    }

    fn set_handler(&mut self, handler: Arc<Mutex<IoHandler>>) -> Result<(), TransportError> {
        let handler = (*handler.lock().unwrap()).clone();

        self.server_builder = Some(ServerBuilder::new(handler));

        Ok(())
    }

    fn start(&mut self) -> Result<(), TransportError> {
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
