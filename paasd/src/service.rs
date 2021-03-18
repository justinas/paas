use std::{convert::TryFrom, pin::Pin};

use futures::stream::Stream;
use tonic::{Request, Response, Status};

use paas_types::process_service_server as server_types;
use paas_types::{
    ExecRequest, ExecResponse, LogsRequest, LogsResponse, StatusRequest, StatusResponse,
    StopRequest, StopResponse,
};

use crate::user::UserId;

pub fn make_server() -> server_types::ProcessServiceServer<ProcessService> {
    server_types::ProcessServiceServer::new(ProcessService::new())
}

#[derive(Clone)]
pub struct ProcessService;

impl ProcessService {
    fn new() -> Self {
        Self
    }

    fn authenticate<T>(req: &Request<T>) -> Result<UserId, Status> {
        let peer_certs = req.peer_certs().expect("no peer certs in the request");
        let cert = peer_certs
            .iter()
            .next()
            .expect("no peer certs in the request");

        Ok(UserId::try_from(cert).map_err(|_| {
            Status::unauthenticated("Could not parse client common name from client certificate")
        })?)
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
        req: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let uid = Self::authenticate(&req)?;
        dbg!(uid);
        Err(Status::unimplemented(""))
    }

    async fn stop(&self, _req: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        Err(Status::unimplemented(""))
    }
}
