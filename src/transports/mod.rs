use std::sync::{Arc, Mutex};

use crate::handler::Handler;

/// A JSON-RPC over HTTP transport built around the `jsonrpc_http_server` library.
#[cfg(feature = "http")]
pub mod http;
/// A JSON-RPC over TCP transport built around the `jsonrpc_tcp_server` library.
#[cfg(feature = "tcp")]
pub mod tcp;
/// A JSON-RPC over WebSockets transport built around the `jsonrpc_ws_server` library.
#[cfg(feature = "ws")]
pub mod ws;

/// Enumerates all the different errors that a `Transport` can get into.
#[derive(Debug)]
pub enum TransportError {
    /// An IP or address cannot be parsed.
    Address(std::net::AddrParseError),
    /// An IO error.
    IO(std::io::Error),
    /// Some operation requires an IO handler, but none is configured yet.
    NoHandler,
    /// An unknown error.
    Unknown,
    /// An error that is specific to WebSockets.
    #[cfg(feature = "ws")]
    WebSockets(Box<crate::transports::ws::Error>),
}

impl From<std::io::Error> for TransportError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<std::net::AddrParseError> for TransportError {
    fn from(value: std::net::AddrParseError) -> Self {
        Self::Address(value)
    }
}

#[cfg(feature = "ws")]
impl From<crate::transports::ws::Error> for TransportError {
    fn from(value: crate::transports::ws::Error) -> Self {
        match value {
            crate::transports::ws::Error::Io(e) => Self::IO(e),
            e => Self::WebSockets(Box::new(e)),
        }
    }
}

/// Generically defines message transports that can be used with JSON-RPC servers.
pub trait Transport<H>
where
    H: Handler,
{
    /// Tell whether this transport requires reconfiguration if the IO handler of the server is
    /// changed.
    ///
    /// That is the case for those transports that use a server builder behind the scenes, because
    /// they cannot leverage the `Arc` around the IO handler of the parent JSON-RPC server.
    fn requires_reset(&self) -> bool;
    /// Tell whether the transport is listening or not.
    fn running(&self) -> bool;
    /// Set the IO handler that the transport will use to process the JSON-RPC messages it receives..
    fn set_handler(&mut self, handler: Arc<Mutex<H>>) -> Result<(), TransportError>;
    /// Start the transport.
    ///
    /// Most often, this will start message listeners, network sockets, and the like.
    fn start(&mut self) -> Result<(), TransportError>;
    /// Stop the transport.
    ///
    /// Stopping a transport is assumed to also stop any underlying listeners and sockets, and to
    /// completely halt the processing of further JSON-RPC messages.
    fn stop(&mut self) -> Result<(), TransportError>;
}
