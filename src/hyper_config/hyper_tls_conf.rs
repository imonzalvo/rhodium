mod certs;
use self::certs::get_configuration;

use core::task::{Context, Poll};
use std::io;
use std::pin::Pin;

use futures_util::stream::*;
use hyper::server::accept::Accept;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;
use tokio_stream::wrappers::TcpListenerStream;

pub struct HyperTlsAcceptor<'a> {
    tls_stream: Pin<Box<dyn Stream<Item = Result<TlsStream<TcpStream>, io::Error>> + 'a>>,
}

impl Accept for HyperTlsAcceptor<'_> {
    type Conn = TlsStream<TcpStream>;
    type Error = io::Error;

    fn poll_accept(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        Pin::new(&mut self.tls_stream).poll_next(cx)
    }
}

impl HyperTlsAcceptor<'_> {
    pub fn new<'a>(
        tcp: TcpListener,
        crt_file: &'a str,
        key_file: &'a str,
    ) -> io::Result<HyperTlsAcceptor<'a>> {
        let server_config = get_configuration(crt_file, key_file)?;
        let tls_acceptor = TlsAcceptor::from(server_config);
        let tls_stream = TcpListenerStream::new(tcp)
            .and_then(move |s| tls_acceptor.accept(s))
            .boxed();

        Ok(HyperTlsAcceptor { tls_stream })
    }
}
