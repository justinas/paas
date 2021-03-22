use paasc::make_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let mut client = make_client(8443, "client1").await?;

    let resp = client
        .exec(paas_types::ExecRequest {
            args: vec!["echo".into(), "foo".into()],
        })
        .await?;
    dbg!(&resp);
    let id = resp.into_inner().id;

    let resp = client.get_status(paas_types::StatusRequest { id }).await?;
    dbg!(&resp);
    Ok(())
}
