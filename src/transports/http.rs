use std::sync::{Arc, Mutex};

use jsonrpc_http_server::{Server, ServerBuilder};

use crate::{
    handler::Handler,
    transports::{Transport, TransportError},
};

/// Settings needed for constructing an `HttpTransport`.
#[derive(Debug)]
pub struct HttpTransportSettings {
    /// An IP or address to bind the HTTP listener to.
    pub address: String,
}

/// A JSON-RPC over HTTP transport built around the `jsonrpc_http_server` library.
pub struct HttpTransport<H>
where
    H: Handler,
{
    settings: HttpTransportSettings,
    server_builder: Option<ServerBuilder<H::Metadata>>,
    server: Option<Server>,
}

impl<H> HttpTransport<H>
where
    H: Handler,
{
    /// Create a new instance of this transport.
    pub fn new(settings: HttpTransportSettings) -> Self {
        Self {
            settings,
            server_builder: None,
            server: None,
        }
    }
}

impl<H> Transport<H> for HttpTransport<H>
where
    H: Handler,
    H::Metadata: Default,
{
    fn requires_reset(&self) -> bool {
        true
    }

    fn running(&self) -> bool {
        self.server.is_some()
    }

    fn set_handler(&mut self, handler: Arc<Mutex<H>>) -> Result<(), TransportError> {
        let handler = (*handler.lock().unwrap()).as_meta_io_handler();
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
            }
        }
    }
}
