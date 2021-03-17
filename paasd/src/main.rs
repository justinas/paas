use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};

mod service;
use service::make_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let cert = tokio::fs::read("data/server.pem").await?;
    let key = tokio::fs::read("data/server.key").await?;
    let identity = Identity::from_pem(cert, key);

    let client_ca_cert = Certificate::from_pem(tokio::fs::read("data/client_ca.pem").await?);

    let tls = ServerTlsConfig::new()
        .identity(identity)
        .client_ca_root(client_ca_cert);

    Server::builder()
        .tls_config(tls)?
        .add_service(make_server())
        .serve("127.0.0.1:8443".parse()?)
        .await?;
    Ok(())
}
