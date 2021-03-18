use std::{convert::TryFrom, os::unix::process::ExitStatusExt, pin::Pin, sync::Arc};

use futures::stream::Stream;
use tonic::{Request, Response, Status};

use paas_types::process_service_server as server_types;
use paas_types::{
    status_response::ExitStatus, ExecRequest, ExecResponse, LogsRequest, LogsResponse,
    StatusRequest, StatusResponse, StopRequest, StopResponse,
};
use worker::Process;

use crate::{store::ProcessStore, user::UserId};
use uuid::Uuid;

pub fn make_server() -> server_types::ProcessServiceServer<ProcessService> {
    server_types::ProcessServiceServer::new(ProcessService::new(Arc::new(ProcessStore::new())))
}

#[derive(Clone)]
pub struct ProcessService {
    store: Arc<ProcessStore>,
}

impl ProcessService {
    fn new(store: Arc<ProcessStore>) -> Self {
        Self { store }
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

    async fn exec(&self, req: Request<ExecRequest>) -> Result<Response<ExecResponse>, Status> {
        let uid = Self::authenticate(&req)?;
        let req = req.into_inner();
        if req.args.is_empty() {
            return Err(Status::invalid_argument("'args' must be a non-empty array"));
        }
        match Process::spawn(&*req.args[0], req.args[1..].iter().map(AsRef::as_ref)) {
            Ok(p) => {
                let pid = self.store.insert(&uid, p);
                Ok(Response::new(ExecResponse {
                    id: Some(pid.into()),
                }))
            }
            Err(e) => Err(Status::unknown(format!("{}", e))),
        }
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
        let req = req.into_inner();
        let raw_pid = req
            .id
            .map(|id| id.id)
            .ok_or_else(|| Status::invalid_argument("Process ID not given"))?;
        let pid =
            Uuid::from_slice(&raw_pid).map_err(|_| Status::invalid_argument("Invalid UUID"))?;
        let status = self
            .store
            .get(pid, &uid)
            .map_err(Into::<Status>::into)?
            .status()
            .await;
        let code = status.and_then(|s| s.code());
        let signal = status.and_then(|s| s.signal());
        Ok(Response::new(StatusResponse {
            exit_status: match (code, signal) {
                (Some(c), None) => Some(ExitStatus::Code(c)),
                (None, Some(s)) => Some(ExitStatus::Signal(s)),
                (None, None) => None,
                _ => unreachable!("Exit code & signal should be mutually exclusive"),
            },
        }))
    }

    async fn stop(&self, _req: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        Err(Status::unimplemented(""))
    }
}
