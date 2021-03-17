use std::{fs::File, io::BufReader};

use rustls::{internal::pemfile, ClientConfig, RootCertStore};
use tonic::transport::Channel;

use paas_types::process_service_client as client;
use paas_types::StatusRequest;

fn buf_read(path: &str) -> Result<BufReader<File>, Box<dyn std::error::Error>> {
    Ok(BufReader::new(File::open(path)?))
}

fn rustls_config() -> Result<ClientConfig, Box<dyn std::error::Error>> {
    let mut cert_store = RootCertStore::empty();
    cert_store
        .add_pem_file(&mut buf_read("data/server_ca.pem")?)
        .unwrap();

    let cert = pemfile::certs(&mut buf_read("data/client1.pem")?).unwrap();
    let key = pemfile::rsa_private_keys(&mut buf_read("data/client1.key")?)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let mut config = ClientConfig::new();
    config.root_store = cert_store;
    config.set_single_client_cert(cert, key)?;
    config.set_protocols(&[b"h2"[..].into()]);
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let tls = tonic::transport::ClientTlsConfig::new().rustls_client_config(rustls_config()?);

    let channel = Channel::from_static("http://localhost:8443")
        .tls_config(tls)?
        .connect()
        .await?;
    let mut client = client::ProcessServiceClient::new(channel);
    let resp = client.get_status(StatusRequest { id: None }).await?;
    dbg!(resp);
    Ok(())
}
