use std::{
    io::{BufRead, BufReader, Write},
    marker::Sync,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

use jsonrpc_core::{Compatibility, Metadata, MetaIoHandler, NoopMiddleware};

use threadpool::ThreadPool;

use crate::{Transport, transports::TransportError};

#[derive(Debug)]
pub struct TcpTransportSettings {
    pub address: String,
}

pub struct TcpTransport<M> where M: Metadata {
    // TODO: Change Mutex for RwLock
    handler: Arc<Mutex<MetaIoHandler<M>>>,
    listener: Option<Arc<TcpListener>>,
    settings: TcpTransportSettings,
}

impl<M> TcpTransport<M> where M: Metadata {
    pub fn new(settings: TcpTransportSettings) -> Self {
        Self {
            handler: Arc::new(Mutex::new(MetaIoHandler::new(Compatibility::V2, NoopMiddleware))),
            listener: None,
            settings,
        }
    }

    fn handle_connection(mut stream: TcpStream, handler: Arc<Mutex<MetaIoHandler<M>>>, meta: M) {
        let mut stream_writer = stream.try_clone().unwrap();
        let buf_reader = BufReader::new(&mut stream);
        for line in buf_reader.lines() {
            let request = line.unwrap();
            let handler = handler.lock().unwrap();

            println!(">> {}", request);

            let response = handler.handle_request_sync(&request, meta.clone());
            if let Some(response) = response {
                println!("<< {}", response);
                stream_writer.write(response.as_bytes()).ok();
                stream_writer.write("\n".as_bytes()).ok();
            }
        }
    }
}

impl<M> core::fmt::Debug for TcpTransport<M> where M: Metadata {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TcpTransport")
            .field("settings", &format_args!("{:?}", self.settings))
            .field("listener", &format_args!("{:?}", self.listener))
            .finish()
    }
}

impl<M> Transport<M> for TcpTransport<M> where M: Metadata + Sync {
    fn requires_reset(&self) -> bool {
        false
    }

    fn running(&self) -> bool {
        self.listener.is_some()
    }

    fn set_handler(&mut self, handler: Arc<Mutex<MetaIoHandler<M>>>) -> Result<(), TransportError> {
        self.handler = handler;

        Ok(())
    }

    fn start(&mut self, meta: M) -> Result<(), TransportError> {
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
                let inner_meta = meta.clone();

                pool.execute(|| {
                    Self::handle_connection(stream, inner_handler, inner_meta);
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
