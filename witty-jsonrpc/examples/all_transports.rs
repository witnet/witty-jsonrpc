#[cfg(feature = "http")]
#[cfg(feature = "tcp")]
#[cfg(feature = "ws")]
pub fn main() {
    use witty_jsonrpc::prelude::*;

    let settings_http = HttpTransportSettings {
        address: "127.0.0.1:9001".into(),
    };
    let transport_http = HttpTransport::new(settings_http);
    let settings_tcp = TcpTransportSettings {
        address: "127.0.0.1:9002".into(),
    };
    let transport_tcp = TcpTransport::new(settings_tcp);
    let settings_ws = WsTransportSettings {
        address: "127.0.0.1:9003".into(),
    };
    let transport_ws = WsTransport::new(settings_ws);

    let mut server = WittyMultiServer::new();

    server.add_transport(transport_http);
    server.add_transport(transport_tcp);
    server.add_transport(transport_ws);

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
