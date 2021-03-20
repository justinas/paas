use std::error::Error;

use paasd::make_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    make_server()?.serve("127.0.0.1:8443".parse()?).await?;
    Ok(())
}
