use async_trait::async_trait;
use hyper::{body::Body, client::connect::HttpConnector, Client, Response, StatusCode};
use hyper_tls::HttpsConnector;
use native_tls::{Certificate, TlsConnector};
use rhodium::{errors::*, request::*, response::*, stack::*, *};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::{thread, time};

// Mock implementations
struct Comm {
    return_error: bool,
}

impl CommunicationChannel for Comm {
    fn new() -> Comm {
        Comm {
            return_error: false,
        }
    }
}

struct Service {}
#[async_trait]
impl RhodService<Comm> for Service {
    async fn serve(
        &self,
        _conn: &RhodConnInfo,
        _req: RhodRequest,
        comm: &mut Comm,
    ) -> RhodResult<RhodResponse> {
        if comm.return_error {
            Err(RhodError::from_str("some error", RhodErrorLevel::Warning))
        } else {
            let res = Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .unwrap();

            let res = RhodResponse::new(res);
            Ok(res)
        }
    }
}

struct ErrorHandler {}
#[async_trait]
impl RhodHandler<Comm> for ErrorHandler {
    async fn handle_request(
        &self,
        _conn: &RhodConnInfo,
        _req: &mut RhodRequest,
        comm: &mut Comm,
    ) -> RhodResult<()> {
        comm.return_error = true;
        Ok(())
    }
    async fn catch_request(
        &self,
        _conn: &RhodConnInfo,
        _req: &RhodRequest,
        _err: &RhodError,
        _comm: &Comm,
    ) {
    }

    async fn handle_response(
        &self,
        _conn: &RhodConnInfo,
        res: RhodResponse,
        _comm: &mut Comm,
    ) -> (RhodResponse, RhodResult<()>) {
        (res, Ok(()))
    }
    async fn catch_response(
        &self,
        _conn: &RhodConnInfo,
        _res: &RhodResponse,
        _err: &RhodError,
        _comm: &Comm,
    ) {
    }
}

fn spawn_rhod(rhod: Rhodium<Comm>) {
    //Create new thread for Rhodium
    thread::spawn(move || {
        use tokio::runtime::Runtime;

        // Create the runtime
        let rt = Runtime::new().unwrap();

        // Execute the future, blocking the current thread until completion
        rt.block_on(rhod.run());
    });

    thread::sleep(time::Duration::from_millis(5000));
}

#[tokio::test]
async fn test_complete_transaction() {
    //create server
    let stack = RhodStack::new(vec![], Box::new(Service {}));
    let rhod = Rhodium::new(
        Arc::new(stack),
        SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), 3000),
        protocols::HttpProtocolConf::HTTP,
    );
    spawn_rhod(rhod);

    //Creates client and gets response
    let client = Client::new();
    let uri = "http://127.0.0.1:3000".parse().unwrap();
    client.get(uri).await.unwrap();
}

#[tokio::test]
async fn test_error_handler() {
    //create server
    let stack = RhodStack::new(
        vec![RhodHandlerInStack::RhodHandler(Box::new(ErrorHandler {}))],
        Box::new(Service {}),
    );
    let rhod = Rhodium::new(
        Arc::new(stack),
        SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), 3001),
        protocols::HttpProtocolConf::HTTP,
    );
    spawn_rhod(rhod);

    //Creates client and gets response
    let client = Client::new();
    let uri = "http://127.0.0.1:3001".parse().unwrap();
    assert!(client.get(uri).await.is_err());
}

#[tokio::test]
async fn test_ssl() {
    //create server
    let stack = RhodStack::new(vec![], Box::new(Service {}));
    let rhod = Rhodium::new(
        Arc::new(stack),
        SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), 3002),
        protocols::HttpProtocolConf::HTTPS {
            cert_file: String::from("tests/assets/certs/server.crt"),
            key_file: String::from("tests/assets/certs/server.key"),
        },
    );
    spawn_rhod(rhod);

    //Reading certificate
    const SELF_SIGNED_CERT: &[u8] = include_bytes!("assets/certs/CA.pem");
    let cert = Certificate::from_pem(SELF_SIGNED_CERT).unwrap();

    //Creating HttpsConnector that trust certificate
    let mut http = HttpConnector::new();
    http.enforce_http(false);
    let mut tls_builder = TlsConnector::builder();
    tls_builder.add_root_certificate(cert); // Adds the certificate to the set of roots that the connector will trust
    let tls = tls_builder.build().unwrap();
    let https = HttpsConnector::from((http, tls.into()));

    //Creates client and gets response
    let client = Client::builder().build::<_, hyper::Body>(https);
    let uri = "https://localhost:3002".parse().unwrap();
    client.get(uri).await.unwrap();
}
