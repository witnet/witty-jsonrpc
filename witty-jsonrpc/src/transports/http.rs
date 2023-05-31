use std::sync::{Arc, Mutex};

use jsonrpc_http_server::{jsonrpc_core::{Metadata, MetaIoHandler}, Server, ServerBuilder};

use crate::{Transport, transports::TransportError};

#[derive(Debug)]
pub struct HttpTransportSettings {
    pub address: String,
}

pub struct HttpTransport<M> where M: Metadata {
    settings: HttpTransportSettings,
    server_builder: Option<ServerBuilder<M>>,
    server: Option<Server>,
}

impl<M> HttpTransport<M> where M: Metadata {
    pub fn new(settings: HttpTransportSettings) -> Self {
        Self {
            settings,
            server_builder: None,
            server: None,
        }
    }
}

impl<M> Transport<M> for HttpTransport<M> where M: Metadata + Default + Unpin {
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
