#[cfg(feature = "http")]
#[cfg(feature = "tcp")]
pub fn main() {
    use std::sync::Arc;

    use jsonrpc_core::Value;

    use witty_jsonrpc::{
        server::{MultipleTransportsServer, Server},
        transports::{
            http::{HttpTransport, HttpTransportSettings},
            tcp::{TcpTransport, TcpTransportSettings},
        },
    };

    let settings_http = HttpTransportSettings {
        address: "127.0.0.1:9001".into(),
    };
    let transport_http = HttpTransport::new(settings_http);
    let settings_tcp = TcpTransportSettings {
        address: "127.0.0.1:9002".into(),
    };
    let transport_tcp = TcpTransport::new(settings_tcp);

    let mut server = MultipleTransportsServer::new(Arc::new(()));

    server.add_transport(transport_http);
    server.add_transport(transport_tcp);

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
