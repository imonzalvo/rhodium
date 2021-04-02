use std::{fs, io};

use std::sync::Arc;

use io::{BufReader, ErrorKind};

use fs::File;

use tokio_rustls::rustls::internal::pemfile;
use tokio_rustls::rustls::{Certificate, NoClientAuth, PrivateKey, ServerConfig};

fn load_certs(filename: &str) -> io::Result<Vec<Certificate>> {
    let cert_file = File::open(filename)?;
    let mut reader = BufReader::new(cert_file);

    pemfile::certs(&mut reader)
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "Couldn't parse certificates"))
}

fn load_private_key(filename: &str) -> io::Result<PrivateKey> {
    let key_file = fs::File::open(filename)?;
    let mut reader = io::BufReader::new(key_file);

    // Load and return a single private key.
    let keys = pemfile::rsa_private_keys(&mut reader)
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "Couldn't parse key"))?;

    if keys.len() != 1 {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "Expected just one key",
        ));
    }

    Ok(keys[0].clone())
}

// Build TLS configuration.
pub fn get_configuration(crt_file: &str, key_file: &str) -> io::Result<Arc<ServerConfig>> {
    let certs = load_certs(crt_file)?;
    let key = load_private_key(key_file)?;

    // Do not use client certificate authentication.
    let mut cfg = ServerConfig::new(NoClientAuth::new());

    cfg.set_single_cert(certs, key).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("Certs and key don't match. {:?}", e),
        )
    })?;

    Ok(Arc::new(cfg))
}
