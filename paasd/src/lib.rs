use std::{fs::File, io::BufReader, sync::Arc};

use anyhow::{anyhow, Error};
use rustls::{
    ciphersuite, internal::pemfile, AllowAnyAuthenticatedClient, RootCertStore, ServerConfig,
    SupportedCipherSuite,
};
use tonic::transport::server::{Router, Server as TonicServer, ServerTlsConfig, Unimplemented};

use paas_types::process_service_server::ProcessServiceServer;

mod service;
mod store;
mod user;

use service::ProcessService;
use store::ProcessStore;

pub type Server = Router<ProcessServiceServer<ProcessService>, Unimplemented>;

static CIPHERSUITES: &[&SupportedCipherSuite; 5] = &[
    &ciphersuite::TLS13_AES_256_GCM_SHA384,
    &ciphersuite::TLS13_CHACHA20_POLY1305_SHA256,
    &ciphersuite::TLS13_AES_128_GCM_SHA256,
    &ciphersuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    &ciphersuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
];

fn buf_read(path: &str) -> Result<BufReader<File>, Error> {
    Ok(BufReader::new(File::open(path)?))
}

fn rustls_config() -> Result<ServerConfig, Error> {
    let mut cert_store = RootCertStore::empty();
    cert_store
        .add_pem_file(&mut buf_read("data/client_ca.pem")?)
        .map_err(|_| anyhow!("could not add client CA to store"))?;

    let cert = pemfile::certs(&mut buf_read("data/server.pem")?)
        .map_err(|_| anyhow!("could not parse the server certificate"))?;
    let keys = pemfile::rsa_private_keys(&mut buf_read("data/server.key")?)
        .map_err(|_| anyhow!("could not parse server private keys"))?;
    let key = keys
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("expected at least one private key"))?;

    let mut config = ServerConfig::new(AllowAnyAuthenticatedClient::new(cert_store));
    config.ciphersuites = CIPHERSUITES.to_vec();
    config.set_single_cert(cert, key)?;
    config.set_protocols(&[b"h2"[..].into()]);
    Ok(config)
}

fn make_service() -> ProcessServiceServer<ProcessService> {
    ProcessServiceServer::new(ProcessService::new(Arc::new(ProcessStore::new())))
}

pub fn make_server() -> Result<Server, Error> {
    let mut tls = ServerTlsConfig::new();
    tls.rustls_server_config(rustls_config()?);

    Ok(TonicServer::builder()
        .tls_config(tls)?
        .add_service(make_service()))
}
