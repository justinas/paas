use rustls::{internal::pemfile, AllowAnyAuthenticatedClient, RootCertStore, ServerConfig};
use std::{fs::File, io::BufReader};
use tonic::transport::Server;

mod service;
use service::make_server;

fn buf_read(path: &str) -> Result<BufReader<File>, Box<dyn std::error::Error>> {
    Ok(BufReader::new(File::open(path)?))
}

fn rustls_config() -> Result<ServerConfig, Box<dyn std::error::Error>> {
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
    config.set_single_cert(cert, key).unwrap();
    config.set_protocols(&[b"h2"[..].into()]);
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let mut tls = tonic::transport::ServerTlsConfig::new();
    tls.rustls_server_config(rustls_config()?);

    Server::builder()
        .tls_config(tls)?
        .add_service(make_server())
        .serve("127.0.0.1:8443".parse()?)
        .await?;
    Ok(())
}
