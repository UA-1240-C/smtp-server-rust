use std::{
    io::ErrorKind, net::TcpStream, pin::Pin, task::{Context, Poll}
};
use futures::{
    io::{AsyncRead, AsyncWrite}, AsyncReadExt, AsyncWriteExt
};

use async_io::Async;
use async_native_tls::{TlsConnector, TlsAcceptor, TlsStream};

pub mod error;
use error::{SmartStreamError, TlsError};

use logger_proc_macro::*;
use logger::{warn, error};

type AsyncTcpStream = Async<std::net::TcpStream>;

pub enum StreamIo<T>
where T: AsyncReadExt + AsyncWriteExt + Unpin {
    Plain(T),
    Encrypted(TlsStream<T>),
}

impl<T> AsyncRead for StreamIo<T>
where T: AsyncReadExt + AsyncWriteExt + Unpin {
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

impl <T> AsyncWrite for StreamIo<T>
where T: AsyncReadExt + AsyncWriteExt + Unpin {
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
        cx: &mut Context<'_>
    ) -> Poll<Result<(), std::io::Error>> {
        match *self {
            Self::Plain(ref mut stream) => Pin::new(stream).poll_flush(cx),
            Self::Encrypted(ref mut stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
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
}

impl AsyncStream {
    #[log(Debug)]
    pub fn new(stream: TcpStream) -> Result<Self, SmartStreamError> {
        let stream = Async::new(stream).map_err(SmartStreamError::from)?;
        Ok(
            Self {
                m_stream: Some(StreamIo::Plain(stream)),
                m_buffsize: 1024,
            }
        )
    }

    #[log(Trace)]
    pub fn close(&mut self) {
        if let Some(stream) =  self.m_stream.as_mut() { 
            match stream {
                StreamIo::Plain(stream) => {
                    let _ = stream.get_ref().shutdown(std::net::Shutdown::Both);
                }
                StreamIo::Encrypted(stream) => {
                    let _ = stream.get_ref().get_ref().shutdown(std::net::Shutdown::Both);
                }
            }
        }
        self.m_stream.take();
    }

    #[log(Trace)]
    pub fn is_open(&self) -> bool {
        match &self.m_stream {
            Some(stream) => {
                let bytes_read_result = match stream {
                    StreamIo::Plain(stream) => stream.get_ref().peek(&mut [0, 8]),
                    StreamIo::Encrypted(stream) => stream.get_ref().get_ref().peek(&mut [0, 8]),
                };

                match bytes_read_result {
                    Ok(bytes_read) => bytes_read > 0,
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        //warn!("No data available yet, try again later.");
                        true
                    }
                    Err(e) => {
                        //error!("Error on check is_open: {:?}", e);
                        false
                    }
                }
            }
            None => false,
        }
    }

    #[log(Trace)]
    pub async fn connect_tls(&mut self) -> Result<(), SmartStreamError> {
        if !self.is_open() {
            return Err(SmartStreamError::ClosedConnection("Error on connect_tls occured".to_string()));
        }

        let stream = self.m_stream.take()
            .ok_or(SmartStreamError::RuntimeError("Error taking stream from option".to_string()))?;

        let connector = TlsConnector::new()
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true);

        let stream = match stream {
            StreamIo::Plain(stream) => {
                let domain = stream.get_ref().peer_addr()?.ip().to_string();
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
            return Err(SmartStreamError::ClosedConnection("Error on accept_tls occured".to_string()));
        }

        let stream = self.m_stream.take()
            .ok_or(SmartStreamError::RuntimeError("Error taking stream from option".to_string()))?;

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
                Some(stream) => stream.write(buf).await.map_err(SmartStreamError::from),
                None => Err(SmartStreamError::RuntimeError("Error getting mutable reference on try to write".to_string())),
            }
        } else {
            Err(SmartStreamError::ClosedConnection("Error on write occured".to_string()))
        }
    }

    #[log(Trace)]
    pub async fn read(&mut self) -> Result<String, SmartStreamError> {
        if self.is_open() {
            Ok(self.read_until_crlf().await?)
        } else {
            Err(SmartStreamError::ClosedConnection("Error on read occured".to_string()))
        }
    }

    #[log(Trace)]
    async fn read_until_crlf(&mut self) -> Result<String, SmartStreamError> {
        let mut response = String::new();
        let mut buffer: Vec<u8> = vec![0; self.m_buffsize as usize];
        let mut bytes_read: usize;

        loop {
            if !self.is_open() {
                break;
            }

            if let Some(stream) = self.m_stream.as_mut() { 
                bytes_read = stream.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
            } else {
                break;
            }
            
            let chunk = &buffer[..bytes_read];
            response.push_str(&String::from_utf8_lossy(chunk));

            if chunk.ends_with(b"\r\n") {
                break;
            }
        }

        Ok(response)
    }
}

impl Drop for AsyncStream {
    #[log(Debug)]
    fn drop(&mut self) {
        if let Some(stream) =  self.m_stream.as_mut() { 
            match stream {
                StreamIo::Plain(stream) => {
                    let _ = stream.get_ref().shutdown(std::net::Shutdown::Both);
                }
                StreamIo::Encrypted(stream) => {
                    let _ = stream.get_ref().get_ref().shutdown(std::net::Shutdown::Both);
                }
            }
        }
        self.m_stream.take();
    }
}
