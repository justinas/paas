use std::{
    convert::{TryFrom, TryInto},
    os::unix::process::ExitStatusExt,
    pin::Pin,
    process::ExitStatus as StdExitStatus,
    sync::Arc,
};

use futures::{stream::Stream, StreamExt};
use tonic::{Request, Response, Status};

use paas_types::process_service_server as server_types;
use paas_types::{
    status_response::ExitStatus, ExecRequest, ExecResponse, LogsRequest, LogsResponse,
    StatusRequest, StatusResponse, StopRequest, StopResponse,
};
use worker::Process;

use crate::{store::ProcessStore, user::UserId};

const NO_PID: &str = "Process ID not given";

fn std_status_to_paas_status(status: StdExitStatus) -> ExitStatus {
    let code = status.code();
    let signal = status.signal();
    match (code, signal) {
        (Some(c), None) => ExitStatus::Code(c),
        (None, Some(s)) => ExitStatus::Signal(s),
        _ => unreachable!("Exit code & signal should be mutually exclusive"),
    }
}

#[derive(Clone)]
pub struct ProcessService {
    store: Arc<ProcessStore>,
}

impl ProcessService {
    pub fn new(store: Arc<ProcessStore>) -> Self {
        Self { store }
    }

    fn authenticate<T>(req: &Request<T>) -> Result<UserId, Status> {
        let peer_certs = req.peer_certs().unwrap_or_else(|| {
            unreachable!("peer certificates should always be present (guaranteed by TLS config)")
        });
        let cert = peer_certs
            .iter()
            .next()
            .unwrap_or_else(|| unreachable!("at least one peer cert should always be present"));

        Ok(UserId::try_from(cert).map_err(Into::<Status>::into)?)
    }

    fn get_process(&self, pid: paas_types::Uuid, uid: &UserId) -> Result<Arc<Process>, Status> {
        let pid = pid
            .try_into()
            .map_err(|_| Status::invalid_argument("Invalid UUID"))?;
        Ok(self.store.get(pid, &uid).map_err(Into::<Status>::into)?)
    }
}

#[tonic::async_trait]
impl server_types::ProcessService for ProcessService {
    type GetLogsStream =
        Pin<Box<dyn Stream<Item = Result<LogsResponse, Status>> + Send + Sync + 'static>>;

    async fn exec(&self, req: Request<ExecRequest>) -> Result<Response<ExecResponse>, Status> {
        let uid = Self::authenticate(&req)?;
        let req = req.into_inner();
        let mut args = req.args.iter();
        let argv0 = args
            .next()
            .ok_or_else(|| Status::invalid_argument("'args' must be a non-empty array"))?;
        match Process::spawn(argv0, args.map(AsRef::as_ref)) {
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
        req: Request<LogsRequest>,
    ) -> Result<Response<Self::GetLogsStream>, Status> {
        let uid = Self::authenticate(&req)?;
        let req = req.into_inner();
        let pid = req.id.ok_or_else(|| Status::invalid_argument(NO_PID))?;
        let process = self.get_process(pid, &uid)?;
        // TODO: buffer multiple lines
        let stream = process.logs().map(|b| Ok(LogsResponse { lines: vec![b] }));
        Ok(Response::new(Box::pin(stream)))
    }

    async fn get_status(
        &self,
        req: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let uid = Self::authenticate(&req)?;
        let req = req.into_inner();
        let pid = req.id.ok_or_else(|| Status::invalid_argument(NO_PID))?;
        let process = self.get_process(pid, &uid)?;
        Ok(Response::new(StatusResponse {
            exit_status: process.status().await.map(std_status_to_paas_status),
        }))
    }

    async fn stop(&self, req: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let uid = Self::authenticate(&req)?;
        let req = req.into_inner();
        let pid = req.id.ok_or_else(|| Status::invalid_argument(NO_PID))?;
        let process = self.get_process(pid, &uid)?;
        match process.stop().await {
            Ok(_) => Ok(Response::new(StopResponse {})),
            // TODO: aborted is a questionable status here
            Err(()) => Err(Status::aborted("Stop operation already in progress")),
        }
    }
}
