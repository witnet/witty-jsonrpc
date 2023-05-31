#[cfg(feature = "tcp")]
pub fn main() {
    use std::sync::Arc;

    use witty_jsonrpc::{
        server::{Server, SingleTransportServer},
        transports::tcp::{TcpTransport, TcpTransportSettings},
    };

    let settings = TcpTransportSettings {
        address: "127.0.0.1:9001".into(),
    };
    let transport = TcpTransport::new(settings);
    let mut server = SingleTransportServer::from_transport(transport, Arc::new(()));
    server.start().unwrap();

    ctrlc::set_handler(|| std::process::exit(0)).unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
