extern crate jsonrpc_pubsub;
extern crate witty_jsonrpc;

#[cfg(feature = "tcp")]
pub fn main() {
    use witty_jsonrpc::prelude::*;

    let settings = TcpTransportSettings {
        address: "127.0.0.1:9001".into(),
    };
    let transport = TcpTransport::new(settings);
    let mut server = WittyMonoServer::from_transport(transport);

    server.start().unwrap();

    ctrlc::set_handler(|| std::process::exit(0)).unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
