// Rhodium allows to create hyper servers as a stack of Handlers. Each Handler has its own handle_request
// and handle_response methods
// Handlers are executed by order while handling a request, and by the reverse order while handling the response.
// The order in which handle_request and handle_response functions are executed is summarized in the next flow diagram:
//
//            -----------            -----------                           -----------            -----------
// --- req -> |         | --- req -> |         | --- req -> ... --- req -> |         | --- req -> |         |
//            | Handler |            | Handler |                           | Handler |            | Service |
//            |    1    |            |    2    |                           |    n    |            |         |
// <-- res -- |_________| <-- res -- |_________| <-- res -- ... <-- res -- |_________| <-- res -- |_________|
//
// Every Handler is a struct implementing de RhodHandler trait, while the Service is a struct implementing the RhodService trait.
// RhodHandlers + RhodService conforms a RhodStack
// To use Rhodium, you just have to create a RhodStack, set the socket address where the hyper server will listen,
// and the protocol to be used (HTTP/HTTPS).
//
//
// If the Handler i returns an error while handling a request:
//      catch_request functions are called for the next handlers (Handler i+1, i+2, ..., n), and then the flow is ended.
// If the Service returns an error:
//      the flow is ended
// If the Handler i returns an error while handling a response:
//      catch_response functions are called for the next handlers (Handler i-1, i-2, ..., 1), and then the flow is ended.

#[macro_use]
extern crate log;

use hyper;
use hyper::server::conn::AddrIncoming;
use hyper::server::conn::AddrStream;
use hyper::Server as HyperServer;

use std::clone::Clone;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;

pub mod errors;
mod hyper_config;
pub mod protocols;
pub mod request;
pub mod response;
pub mod stack;
use self::errors::RhodHyperError; //Server errors (Hyper errors, bad certificates, etc)
use self::hyper_config::*;
use self::protocols::*;
use self::request::*;
use self::stack::*;

// =====================================================================
// ||          Structs to share information between handlers          ||
// =====================================================================

#[derive(Clone)]
pub struct RhodConnInfo {
    pub addr: SocketAddr,
    pub proto: HttpProtocol,
}

impl RhodConnInfo {
    pub fn new(addr: SocketAddr, proto: HttpProtocol) -> RhodConnInfo {
        RhodConnInfo { addr, proto }
    }
}

// A generic type that implements the CommunicationChannel trait will be used for communication between handlers and the service
// Users of the library have to define their CommunicationChannel type, which have to implement this trait.
pub trait CommunicationChannel: Send + Sync + 'static {
    fn new() -> Self;
}

// ==============================
// ||         Rhodium          ||
// ==============================

// Rhodium: has all information needed to run a server
pub struct Rhodium<C: CommunicationChannel> {
    stack: Arc<RhodStack<C>>,   // stack of handlers and the service to execute
    addr: SocketAddr,           // address to listen
    protocol: HttpProtocolConf, // use http or https
}

impl<C: CommunicationChannel> Rhodium<C> {
    pub fn new(
        stack: Arc<RhodStack<C>>,
        addr: SocketAddr,
        protocol: HttpProtocolConf,
    ) -> Rhodium<C> {
        Rhodium {
            stack,
            addr,
            protocol,
        }
    }

    //Creates hyper server that runs the rhodium stack
    pub async fn run(self) -> Result<(), RhodHyperError> {
        println!("Listening on {}://{}", self.protocol.to_string(), self.addr);
        info!("Listening on {}://{}", self.protocol.to_string(), self.addr);

        match &self.protocol {
            HttpProtocolConf::HTTP => {
                match AddrIncoming::bind(&self.addr) {
                    Ok(addr_incoming) => {
                        let builder = HyperServer::builder(addr_incoming);

                        // creating a service factory.
                        // for each request, it will return a RhodHyperService with the rhodium stack, and the connection info (source addr + protocol used)
                        let mk_service = hyper::service::make_service_fn(|socket: &AddrStream| {
                            let stack = Arc::clone(&self.stack);
                            let addr = socket.remote_addr();
                            async move {
                                Ok::<_, RhodHyperError>(RhodHyperService::new(
                                    stack,
                                    RhodConnInfo::new(addr, HttpProtocol::HTTP),
                                ))
                            }
                        });

                        // starts a server with the created service factory
                        // wrapps the Hyper result in a Rhod Hyper result
                        RhodHyperError::from_hyper_error_result(builder.serve(mk_service).await)
                    }
                    Err(e) => Err(RhodHyperError::ConfigError(format!(
                        "Error when binding (HTTP). {}",
                        e
                    ))),
                }
            }
            HttpProtocolConf::HTTPS {
                cert_file,
                key_file,
            } => {
                // Create a TCP listener via tokio.
                match TcpListener::bind(&self.addr).await {
                    Ok(tcp) => match HyperTlsAcceptor::new(tcp, &cert_file, &key_file) {
                        Ok(tls_acceptor) => {
                            let builder = HyperServer::builder(tls_acceptor);

                            // creating a service factory.
                            // for each request, it will return a RhodHyperService with the rhodium stack, and the connection info (source addr + protocol used)
                            let mk_service =
                                hyper::service::make_service_fn(|stream: &TlsStream<TcpStream>| {
                                    let stack = Arc::clone(&self.stack);
                                    let addr = stream.get_ref().0.peer_addr();
                                    async move {
                                        match addr {
                                            Ok(peer_addr) => {
                                                Ok::<_, RhodHyperError>(RhodHyperService::new(
                                                    stack,
                                                    RhodConnInfo::new(
                                                        peer_addr,
                                                        HttpProtocol::HTTPS,
                                                    ),
                                                ))
                                            }
                                            Err(e) => Err::<RhodHyperService<C>, RhodHyperError>(
                                                RhodHyperError::ConfigError(format!(
                                                    "Couldnt parse client IP. {}",
                                                    e
                                                )),
                                            ),
                                        }
                                    }
                                });

                            // starts a server with the created service factory
                            // wrapps the Hyper result in a Rhod Hyper result
                            RhodHyperError::from_hyper_error_result(builder.serve(mk_service).await)
                        }
                        Err(e) => Err(RhodHyperError::ConfigError(format!(
                            "Error when creating TLS Acceptor. {}",
                            e
                        ))),
                    },
                    Err(e) => Err(RhodHyperError::ConfigError(format!(
                        "Error when binding (HTTPS). {}",
                        e
                    ))),
                }
            }
        }
    }
}
