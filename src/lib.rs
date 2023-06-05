pub mod handler;
pub mod server;
pub mod transports;

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
