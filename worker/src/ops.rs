use std::time::Duration;

use nix::{
    sys::signal::{self, kill},
    unistd::Pid,
};
use tokio::{process::Child, time::timeout};

const SIGTERM_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, thiserror::Error)]
pub enum StopError {
    #[error("I/O error while stopping the process: {0}")]
    Io(#[from] std::io::Error),
    #[error("Syscall error while stopping the process: {0}")]
    Nix(#[from] nix::Error),
}

pub async fn stop_child(child: &mut Child) -> Result<(), StopError> {
    let pid = match child.id() {
        Some(id) => id,
        None => return Ok(()),
    } as i32; // TODO: dubious cast? pid_t is i32 in `nix`, but u32 in `std`.
    kill(Pid::from_raw(pid), signal::SIGTERM)?;
    if timeout(SIGTERM_TIMEOUT, child.wait()).await.is_ok() {
        return Ok(());
    }
    Ok(child.kill().await?)
}
