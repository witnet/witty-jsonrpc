use std::sync::{Arc, Mutex};

use crate::handler::Handler;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "tcp")]
pub mod tcp;
#[cfg(feature = "ws")]
pub mod ws;

#[derive(Debug)]
pub enum TransportError {
    Address(std::net::AddrParseError),
    IO(std::io::Error),
    NoHandler,
    Unknown,
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

pub trait Transport<H>
where
    H: Handler,
{
    fn requires_reset(&self) -> bool;
    fn running(&self) -> bool;
    fn set_handler(&mut self, handler: Arc<Mutex<H>>) -> Result<(), TransportError>;
    fn start(&mut self) -> Result<(), TransportError>;
    fn stop(&mut self) -> Result<(), TransportError>;
}
