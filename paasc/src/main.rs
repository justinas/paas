use futures::{pin_mut, stream::StreamExt};
use paasc::make_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let mut client = make_client(8443, "client1").await?;

    let resp = client
        .exec(paas_types::ExecRequest {
            args: vec![
                "bash".into(),
                "-c".into(),
                "while true; do echo $RANDOM; sleep 1; done".into(),
            ],
            //args: vec!["echo".into(), "foo".into()],
        })
        .await?;
    dbg!(&resp);
    let id = resp.into_inner().id;

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let stream = client
        .get_logs(paas_types::LogsRequest { id: id.clone() })
        .await?
        .into_inner();

    pin_mut!(stream);

    stream
        .for_each(|m| async move {
            println!("{:?}", m);
        })
        .await;

    /*
    let resp = client
        .get_status(paas_types::StatusRequest { id: id.clone() })
        .await?;
    dbg!(&resp);

    let resp = client
        .stop(paas_types::StopRequest { id: id.clone() })
        .await?;
    dbg!(&resp);

    let resp = client
        .get_status(paas_types::StatusRequest { id: id.clone() })
        .await?;
    dbg!(&resp);
    */
    Ok(())
}
