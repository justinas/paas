use futures::stream::Stream;
use paas_types::process_service_server as server_types;
use paas_types::{
    ExecRequest, ExecResponse, LogsRequest, LogsResponse, StatusRequest, StatusResponse,
    StopRequest, StopResponse,
};
use std::pin::Pin;
use tonic::{
    transport::{Certificate, Identity, Server, ServerTlsConfig},
    Request, Response, Status,
};

#[derive(Clone)]
struct ProcessService;

impl ProcessService {
    fn new() -> Self {
        Self
    }
}

#[tonic::async_trait]
impl server_types::ProcessService for ProcessService {
    type GetLogsStream =
        Pin<Box<dyn Stream<Item = Result<LogsResponse, tonic::Status>> + Send + Sync + 'static>>;

    async fn exec(&self, _req: Request<ExecRequest>) -> Result<Response<ExecResponse>, Status> {
        Err(Status::unimplemented(""))
    }

    async fn get_logs(
        &self,
        _req: Request<LogsRequest>,
    ) -> Result<Response<Self::GetLogsStream>, Status> {
        Err(Status::unimplemented(""))
    }

    async fn get_status(
        &self,
        _req: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        Err(Status::unimplemented(""))
    }

    async fn stop(&self, _req: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        Err(Status::unimplemented(""))
    }
}

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

    let service = ProcessService::new();
    Server::builder()
        .tls_config(tls)?
        .add_service(server_types::ProcessServiceServer::new(service))
        .serve("127.0.0.1:8443".parse()?)
        .await?;
    Ok(())
}
