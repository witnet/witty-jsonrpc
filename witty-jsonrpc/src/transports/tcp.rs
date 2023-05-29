use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

use jsonrpc_core::IoHandler;
use threadpool::ThreadPool;

use crate::{transports::TransportError, Transport};

#[derive(Debug)]
pub struct TcpTransportSettings {
    pub address: String,
}

pub struct TcpTransport {
    settings: TcpTransportSettings,
    listener: Option<Arc<TcpListener>>,
    handler: Arc<Mutex<IoHandler>>,
}

impl TcpTransport {
    pub fn new(settings: TcpTransportSettings) -> Self {
        Self {
            settings,
            listener: None,
            handler: Arc::new(Mutex::new(IoHandler::new())),
        }
    }

    fn handle_connection(mut stream: TcpStream, handler: Arc<Mutex<IoHandler>>) {
        let mut stream_writer = stream.try_clone().unwrap();
        let buf_reader = BufReader::new(&mut stream);
        for line in buf_reader.lines() {
            let request = line.unwrap();
            let handler = handler.lock().unwrap();

            let response = handler.handle_request_sync(&request);
            if let Some(response) = response {
                stream_writer.write(response.as_bytes()).ok();
                stream_writer.write("\n".as_bytes()).ok();
            }
        }
    }
}

impl core::fmt::Debug for TcpTransport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TcpTransport")
            .field("settings", &format_args!("{:?}", self.settings))
            .field("listener", &format_args!("{:?}", self.listener))
            .finish()
    }
}

impl Transport for TcpTransport {
    fn requires_reset(&self) -> bool {
        false
    }

    fn running(&self) -> bool {
        self.listener.is_some()
    }

    fn set_handler(&mut self, handler: Arc<Mutex<IoHandler>>) -> Result<(), TransportError> {
        self.handler = handler;

        Ok(())
    }

    fn start(&mut self) -> Result<(), TransportError> {
        if self.listener.is_some() {
            return Ok(());
        }

        let listener = Arc::new(TcpListener::bind(self.settings.address.clone())?);
        let cloned_listener = listener.clone();
        let cloned_handler = self.handler.clone();

        thread::spawn(move || {
            let pool = ThreadPool::new(4);
            for stream in cloned_listener.incoming() {
                let stream = stream.unwrap();
                let inner_handler = cloned_handler.clone();

                pool.execute(|| {
                    Self::handle_connection(stream, inner_handler);
                });
            }
        });

        self.listener = Some(listener);

        Ok(())
    }

    fn stop(&mut self) -> Result<(), TransportError> {
        self.listener.take().ok_or(TransportError::NoHandler)?;

        Ok(())
    }
}
