use anyhow::Error;
use log::info;

use paasd::make_server;

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let addr = "127.0.0.1:8443".parse()?;
    info!("starting on {}", addr);
    make_server()?.serve(addr).await?;
    Ok(())
}
