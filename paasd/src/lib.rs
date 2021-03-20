use std::{error::Error, fs::File, io::BufReader, sync::Arc};

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

fn buf_read(path: &str) -> Result<BufReader<File>, Box<dyn Error>> {
    Ok(BufReader::new(File::open(path)?))
}

fn rustls_config() -> Result<ServerConfig, Box<dyn Error>> {
    let mut cert_store = RootCertStore::empty();
    cert_store
        .add_pem_file(&mut buf_read("data/client_ca.pem")?)
        .unwrap();

    let cert = pemfile::certs(&mut buf_read("data/server.pem")?).unwrap();
    let key = pemfile::rsa_private_keys(&mut buf_read("data/server.key")?)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let mut config = ServerConfig::new(AllowAnyAuthenticatedClient::new(cert_store));
    config.ciphersuites = CIPHERSUITES.to_vec();
    config.set_single_cert(cert, key).unwrap();
    config.set_protocols(&[b"h2"[..].into()]);
    Ok(config)
}

fn make_service() -> ProcessServiceServer<ProcessService> {
    ProcessServiceServer::new(ProcessService::new(Arc::new(ProcessStore::new())))
}

pub fn make_server() -> Result<Server, Box<dyn Error>> {
    let mut tls = ServerTlsConfig::new();
    tls.rustls_server_config(rustls_config()?);

    Ok(TonicServer::builder()
        .tls_config(tls)?
        .add_service(make_service()))
}
