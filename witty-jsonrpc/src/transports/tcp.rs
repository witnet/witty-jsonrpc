use std::sync::{Arc, Mutex};

use jsonrpc_core::NoopMiddleware;
use jsonrpc_tcp_server::{RequestContext, Server, ServerBuilder};

use crate::{
    handler::Handler,
    transports::{Transport, TransportError},
};

#[derive(Debug)]
pub struct TcpTransportSettings {
    pub address: String,
}

pub struct TcpTransport<H>
where
    H: Handler,
{
    settings: TcpTransportSettings,
    server_builder: Option<ServerBuilder<H::Metadata, NoopMiddleware>>,
    server: Option<Server>,
}

impl<H> TcpTransport<H>
where
    H: Handler,
{
    pub fn new(settings: TcpTransportSettings) -> Self {
        Self {
            settings,
            server_builder: None,
            server: None,
        }
    }
}

impl<H> Transport<H> for TcpTransport<H>
where
    H: Handler + Send + 'static,
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
                H::metadata_from_sender(context.sender.clone())
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
