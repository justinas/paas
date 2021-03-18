use structopt::StructOpt;
use uuid::Uuid;

use paas_types::process_service_client as client;
use paasc::make_client;

mod ops;

#[derive(Debug, StructOpt)]
enum Opt {
    #[structopt(about = "Execute a process")]
    Exec {
        #[structopt(help = "Argument list")]
        args: Vec<String>,
    },
    #[structopt(about = "Stream logs of the process with the given UUID")]
    Logs {
        #[structopt(help = "UUID of the process")]
        pid: Uuid,
    },
    #[structopt(about = "Get status of the process with the given UUID")]
    Status {
        #[structopt(help = "UUID of the process")]
        pid: Uuid,
    },
    #[structopt(
        about = "Stop the process with the given UUID. If process has already finished, has no effect."
    )]
    Stop {
        #[structopt(help = "UUID of the process")]
        pid: Uuid,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    pretty_env_logger::init();

    let opt = Opt::from_args();

    let mut client = make_client(8443, "client1").await?;

    match opt {
        Opt::Exec { args } if args.is_empty() => {
            anyhow::bail!("Empty process argument line");
        }
        Opt::Exec { args } => ops::exec(client, args).await,
        Opt::Logs { pid } => ops::logs(client, pid).await,
        Opt::Status { pid } => ops::status(client, pid).await,
        Opt::Stop { pid } => ops::stop(client, pid).await,
    }?;
    Ok(())
}
