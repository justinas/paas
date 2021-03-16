use paas_types::process_service_client as client;
use paas_types::StatusRequest;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let cert = tokio::fs::read("data/client2.pem").await?;
    let key = tokio::fs::read("data/client2.key").await?;
    let identity = Identity::from_pem(cert, key);

    let server_ca_cert = tokio::fs::read("data/server_ca.pem").await?;

    let tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(Certificate::from_pem(server_ca_cert))
        .identity(identity);

    let channel = Channel::from_static("http://localhost:8443")
        .tls_config(tls)?
        .connect()
        .await?;
    let mut client = client::ProcessServiceClient::new(channel);
    let resp = client.get_status(StatusRequest { id: None }).await?;
    dbg!(resp);
    Ok(())
}
