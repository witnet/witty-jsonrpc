use std::sync::{Arc, Mutex};

pub use jsonrpc_ws_server::Error;
use jsonrpc_ws_server::{jsonrpc_core::NoopMiddleware, RequestContext, Server, ServerBuilder};

use crate::{
    handler::Handler,
    transports::{Transport, TransportError},
};

/// Settings needed for constructing a `WsTransport`.
pub struct WsTransportSettings {
    /// An IP or address to bind the WebSockets listener to.
    pub address: String,
}

/// A JSON-RPC over WebSockets transport built around the `jsonrpc_ws_server` library.
pub struct WsTransport<H>
where
    H: Handler,
{
    settings: WsTransportSettings,
    server_builder: Option<ServerBuilder<H::Metadata, NoopMiddleware>>,
    server: Option<Server>,
}

impl<H> WsTransport<H>
where
    H: Handler,
{
    /// Create a new instance of this transport.
    pub fn new(settings: WsTransportSettings) -> Self {
        Self {
            settings,
            server_builder: None,
            server: None,
        }
    }
}

impl<H> Transport<H> for WsTransport<H>
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
        let server_builder =
            ServerBuilder::new(handler).session_meta_extractor(|context: &RequestContext| {
                H::metadata_from_sender(context.sender())
            });
        self.server_builder = Some(server_builder);

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
            }
        }
    }
}
