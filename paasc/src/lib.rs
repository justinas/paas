use anyhow::{anyhow, Error};
use rustls::{ciphersuite, internal::pemfile, ClientConfig, RootCertStore, SupportedCipherSuite};
use std::{fs::File, io::BufReader, path::Path};
use tonic::transport::Channel;

use paas_types::process_service_client::ProcessServiceClient;

static CIPHERSUITES: &[&SupportedCipherSuite; 5] = &[
    &ciphersuite::TLS13_AES_256_GCM_SHA384,
    &ciphersuite::TLS13_CHACHA20_POLY1305_SHA256,
    &ciphersuite::TLS13_AES_128_GCM_SHA256,
    &ciphersuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    &ciphersuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
];

fn buf_read(path: impl AsRef<Path>) -> Result<BufReader<File>, Error> {
    Ok(BufReader::new(File::open(path)?))
}

fn rustls_config(client: &str) -> Result<ClientConfig, Error> {
    let data_path = Path::new("./data");
    let mut cert_store = RootCertStore::empty();
    cert_store
        .add_pem_file(&mut buf_read(data_path.join("server_ca.pem"))?)
        .map_err(|_| anyhow!("could not add server CA to store"))?;

    let cert_path = data_path.join(&format!("{}.pem", client));
    let key_path = data_path.join(&format!("{}.key", client));
    let cert = pemfile::certs(&mut buf_read(&cert_path)?)
        .map_err(|_| anyhow!("could not parse the client certificate"))?;
    let keys = pemfile::rsa_private_keys(&mut buf_read(&key_path)?)
        .map_err(|_| anyhow!("could not parse server private keys"))?;
    let key = keys
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("expected at least one private key"))?;

    let mut config = ClientConfig::new();
    config.ciphersuites = CIPHERSUITES.to_vec();
    config.root_store = cert_store;
    config.set_single_client_cert(cert, key)?;
    config.set_protocols(&[b"h2"[..].into()]);
    Ok(config)
}

pub async fn make_client(port: u16, client: &str) -> Result<ProcessServiceClient<Channel>, Error> {
    let tls = tonic::transport::ClientTlsConfig::new().rustls_client_config(rustls_config(client)?);

    let channel = Channel::from_shared(format!("https://localhost:{}", port))?
        .tls_config(tls)?
        .connect()
        .await?;
    Ok(ProcessServiceClient::new(channel))
}
