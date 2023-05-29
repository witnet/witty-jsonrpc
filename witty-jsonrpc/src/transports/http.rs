use std::sync::{Arc, Mutex};

use jsonrpc_http_server::{jsonrpc_core::IoHandler, Server, ServerBuilder};

use crate::{transports::TransportError, Transport};

#[derive(Debug)]
pub struct HttpTransportSettings {
    pub address: String,
}

pub struct HttpTransport {
    settings: HttpTransportSettings,
    server_builder: Option<ServerBuilder>,
    server: Option<Server>,
}

impl HttpTransport {
    pub fn new(settings: HttpTransportSettings) -> Self {
        Self {
            settings,
            server_builder: None,
            server: None,
        }
    }
}

impl Transport for HttpTransport {
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
        if self.server.is_some() {
            return Ok(());
        }

        let builder = self
            .server_builder
            .take()
            .ok_or(TransportError::NoHandler)?;
        let socket_addr = self.settings.address.parse::<std::net::SocketAddr>()?;
        self.server = Some(builder.start_http(&socket_addr)?);

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
