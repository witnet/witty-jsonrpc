//! # witty-jsonrpc
//!
//! An extensible JSON-RPC server that can listen over multiple transports at once.
//!
//! ## Supported transports
//! - HTTP
//! - TCP sockets
//! - WebSockets
//! - Whatever `T` you do `impl<H> Transport<H> for T where H: Handler`

#![deny(rust_2018_idioms)]
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![deny(missing_docs)]

/// Traits and implementations enabling compatibility with different IO handlers.
pub mod handler;
/// Traits and implementations of mono-transport and multi-transport servers.
pub mod server;
/// Traits and implementations of message transports (e.g. HTTP, TCP, WS, etc.)
pub mod transports;

/// Make it easy for 3rd party projects to import all the right structures and traits to start using
/// this library immediately.
pub mod prelude {
    pub use jsonrpc_core::Value;
    pub use jsonrpc_pubsub::PubSubHandler;

    #[cfg(feature = "http")]
    pub use crate::transports::http::{HttpTransport, HttpTransportSettings};
    #[cfg(feature = "tcp")]
    pub use crate::transports::tcp::{TcpTransport, TcpTransportSettings};
    #[cfg(feature = "ws")]
    pub use crate::transports::ws::{WsTransport, WsTransportSettings};
    pub use crate::{
        handler::Session,
        server::{
            MultipleTransportsServer, Server, SingleTransportServer, WittyMonoServer,
            WittyMultiServer,
        },
    };
}
