use std::{
    net::TcpStream,
    pin::Pin,
    task::{Context, Poll},
};

use async_native_tls::{TlsAcceptor, TlsConnector, TlsStream};
use async_std::{
    future::timeout,
    io::{Read, ReadExt, Write, WriteExt},
    net::TcpStream as AsyncTcpStream,
};

pub mod error;
use error::{SmartStreamError, TlsError};

use logger_proc_macro::*;

pub enum StreamIo<T>
where
    T: Read + Write + Unpin,
{
    Plain(T),
    Encrypted(TlsStream<T>),
}

impl<T> Read for StreamIo<T>
where
    T: Read + Write + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match *self {
            Self::Plain(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Encrypted(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl<T> Write for StreamIo<T>
where
    T: Read + Write + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match *self {
            Self::Plain(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Encrypted(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match *self {
            Self::Plain(ref mut stream) => Pin::new(stream).poll_flush(cx),
            Self::Encrypted(ref mut stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        match *self {
            Self::Plain(ref mut stream) => Pin::new(stream).poll_close(cx),
            Self::Encrypted(ref mut stream) => Pin::new(stream).poll_close(cx),
        }
    }
}

pub struct AsyncStream {
    m_stream: Option<StreamIo<AsyncTcpStream>>,
    m_buffsize: u16,
    m_timeout: u64,
}

impl AsyncStream {
    #[log(Debug)]
    pub fn new(stream: TcpStream, timeout: u64) -> Result<Self, SmartStreamError> {
        let stream = AsyncTcpStream::from(stream);
        Ok(Self {
            m_stream: Some(StreamIo::Plain(stream)),
            m_buffsize: 1024,
            m_timeout: timeout,
        })
    }

    #[log(Trace)]
    pub fn close(&mut self) {
        if let Some(stream) = self.m_stream.as_mut() {
            match stream {
                StreamIo::Plain(stream) => {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
                StreamIo::Encrypted(stream) => {
                    let _ = stream.get_ref().shutdown(std::net::Shutdown::Both);
                }
            }
        }
        self.m_stream.take();
    }

    #[log(Trace)]
    pub fn is_open(&self) -> bool {
        match &self.m_stream {
            Some(stream) => match stream {
                StreamIo::Plain(stream) => stream.peer_addr().is_ok(),
                StreamIo::Encrypted(stream) => stream.get_ref().peer_addr().is_ok(),
            },
            None => false,
        }
    }

    #[log(Trace)]
    pub async fn connect_tls(&mut self) -> Result<(), SmartStreamError> {
        if !self.is_open() {
            return Err(SmartStreamError::ClosedConnection(
                "Error on connect_tls occured".to_string(),
            ));
        }

        let stream = self.m_stream.take().ok_or(SmartStreamError::RuntimeError(
            "Error taking stream from option".to_string(),
        ))?;

        let connector = TlsConnector::new()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true);

        let stream = match stream {
            StreamIo::Plain(stream) => {
                let domain = stream.peer_addr()?.ip().to_string();
                let stream = connector.connect(domain, stream).await?;
                StreamIo::Encrypted(stream)
            }
            StreamIo::Encrypted(_stream) => {
                return Err(SmartStreamError::Tls(TlsError::StreamAlreadyEncrypted));
            }
        };

        self.m_stream = Some(stream);
        Ok(())
    }

    #[log(Trace)]
    pub async fn accept_tls(&mut self, acceptor: &TlsAcceptor) -> Result<(), SmartStreamError> {
        if !self.is_open() {
            return Err(SmartStreamError::ClosedConnection(
                "Error on accept_tls occured".to_string(),
            ));
        }

        let stream = self.m_stream.take().ok_or(SmartStreamError::RuntimeError(
            "Error taking stream from option".to_string(),
        ))?;

        let stream = match stream {
            StreamIo::Plain(stream) => {
                let stream = acceptor.accept(stream).await?;
                StreamIo::Encrypted(stream)
            }
            StreamIo::Encrypted(_stream) => {
                return Err(SmartStreamError::Tls(TlsError::StreamAlreadyEncrypted));
            }
        };

        self.m_stream = Some(stream);
        Ok(())
    }

    #[log(Trace)]
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, SmartStreamError> {
        if self.is_open() {
            match self.m_stream.as_mut() {
                Some(stream) => stream
                    .write(buf.as_ref())
                    .await
                    .map_err(SmartStreamError::from),
                None => Err(SmartStreamError::RuntimeError(
                    "Error getting mutable reference on try to write".to_string(),
                )),
            }
        } else {
            Err(SmartStreamError::ClosedConnection(
                "Error on write occured".to_string(),
            ))
        }
    }

    #[log(Trace)]
    pub async fn read_until(&mut self, expected_delimiter: &str) -> Result<String, SmartStreamError> {
        if self.is_open() {
            if let Some(stream) = self.m_stream.as_mut() {
                let mut response = Vec::new();

                let mut chunk = vec![0; self.m_buffsize as usize];

                loop {
                    let n = timeout(std::time::Duration::from_secs(self.m_timeout), stream.read(&mut chunk)).await??;

                    response.extend_from_slice(&chunk[..n]);

                    if String::from_utf8(response.clone())?
                        .ends_with(expected_delimiter) {
                        break;
                    }
                }

                Ok(String::from_utf8(response).unwrap())
            } else {
                Err(SmartStreamError::RuntimeError(
                    "Error getting mutable reference on try to read".to_string(),
                ))
            }
        } else {
            Err(SmartStreamError::ClosedConnection(
                "Error on read_until_crlf occured".to_string(),
            ))
        }
    }
}

impl Drop for AsyncStream {
    #[log(Debug)]
    fn drop(&mut self) {
        if let Some(stream) = self.m_stream.as_mut() {
            match stream {
                StreamIo::Plain(stream) => {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
                StreamIo::Encrypted(stream) => {
                    let _ = stream.get_ref().shutdown(std::net::Shutdown::Both);
                }
            }
        }
        self.m_stream.take();
    }
}
