use std::time::Duration;

use nix::{
    sys::signal::{self, kill},
    unistd::Pid,
};
use tokio::{process::Child, time::timeout};

const SIGTERM_TIMEOUT: Duration = Duration::from_secs(5);

// TODO: a better error type
pub async fn stop_child(child: &mut Child) -> Result<(), Box<dyn std::error::Error>> {
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
