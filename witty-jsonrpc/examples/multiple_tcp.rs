#[cfg(feature = "tcp")]
pub fn main() {
    use std::sync::Arc;

    use jsonrpc_core::Value;

    use witty_jsonrpc::{
        server::{MultipleTransportsServer, Server},
        transports::tcp::{TcpTransport, TcpTransportSettings},
    };

    let settings_a = TcpTransportSettings {
        address: "127.0.0.1:9001".into(),
    };
    let transport_a = TcpTransport::new(settings_a);
    let settings_b = TcpTransportSettings {
        address: "127.0.0.1:9002".into(),
    };
    let transport_b = TcpTransport::new(settings_b);

    let mut server = MultipleTransportsServer::new(Arc::new(()));
    server.add_transport(transport_a);
    server.add_transport(transport_b);

    server.add_method("say_hello", |params| {
        println!("Gotcha! Gonna say hello with params {:?}", params);
        futures::future::ok(Value::String(format!(
            "Hello, World! Your params are {:?}",
            params
        )))
    });

    server.start().unwrap();

    ctrlc::set_handler(|| std::process::exit(0)).unwrap();
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
