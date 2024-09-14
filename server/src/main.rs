use concurrent_runtime::ConcurrentRuntime;
use smart_stream::AsyncStream;
use std::sync::Arc;

use async_native_tls::TlsAcceptor;
use native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};

use std::net::TcpListener;
mod config;

use logger::info;

fn main() {
    logger::set_logger_level(logger::LogLevel::Debug);
    logger::set_logger_target(Box::new(logger::ConsoleLogTarget));

    let cfg = config::Config::default();

    logger::set_logger_level(cfg.log_level);
    logger::set_logger_target(cfg.log_target);
    logger::set_logger_cache_capacity(cfg.capacity);

    let mut runtime = ConcurrentRuntime::new(cfg.pool_size);
    runtime.start();
    
    let listener = TcpListener::bind(format!("{}:{}", cfg.ip, cfg.port)).unwrap();
    let native_tls_acceptor: NativeTlsAcceptor = NativeTlsAcceptor::new(
        Identity::from_pkcs8(
            include_bytes!("../certs/server.crt"),
            include_bytes!("../certs/server.key"),
        ).unwrap(),
    ).unwrap();

    let acceptor = Arc::new(TlsAcceptor::from(native_tls_acceptor));
    loop {
        let (stream, _) = listener.accept().unwrap();
        let async_stream = AsyncStream::new(stream).unwrap();
        let acceptor = acceptor.clone();

        runtime.spawn(async move {
            let mut connection = client_connection::ClientConnection::new(async_stream, &acceptor);
            match connection.run().await {
                Ok(_) => info!("Connection closed"),
                Err(e) => info!("Connection error: {:?}", e),
            }
        });

    }
}
