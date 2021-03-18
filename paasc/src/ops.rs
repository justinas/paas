use std::convert::TryInto;

use futures::{pin_mut, stream::StreamExt};
use tonic::transport::Channel;
use uuid::Uuid;

use paas_types::process_service_client::ProcessServiceClient;
use paas_types::status_response::ExitStatus;
use paas_types::{ExecRequest, LogsRequest, StatusRequest, StopRequest};

pub async fn exec(
    mut client: ProcessServiceClient<Channel>,
    args: Vec<String>,
) -> Result<(), anyhow::Error> {
    let resp = client.exec(ExecRequest { args }).await?.into_inner();
    println!(
        "{}",
        TryInto::<Uuid>::try_into(resp.id.unwrap())?.to_hyphenated()
    );
    Ok(())
}

pub async fn logs(
    mut client: ProcessServiceClient<Channel>,
    id: Uuid,
) -> Result<(), anyhow::Error> {
    let stream = client
        .get_logs(LogsRequest {
            id: Some(id.into()),
        })
        .await?
        .into_inner();

    pin_mut!(stream);
    while let Some(resp) = stream.next().await {
        let resp = resp?;
        for l in resp.lines {
            println!("{}", std::str::from_utf8(&l)?);
        }
    }

    Ok(())
}

pub async fn status(
    mut client: ProcessServiceClient<Channel>,
    id: Uuid,
) -> Result<(), anyhow::Error> {
    let resp = client
        .get_status(StatusRequest {
            id: Some(id.into()),
        })
        .await?
        .into_inner();
    match resp.exit_status {
        None => println!("Status: running"),
        Some(ExitStatus::Code(c)) => println!("Status: exited (code {})", c),
        Some(ExitStatus::Signal(s)) => println!("Status: exited (signal {})", s),
    };
    Ok(())
}

pub async fn stop(
    mut client: ProcessServiceClient<Channel>,
    id: Uuid,
) -> Result<(), anyhow::Error> {
    client
        .stop(StopRequest {
            id: Some(id.into()),
        })
        .await?;
    Ok(())
}
