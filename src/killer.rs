//! Process termination with structured error classification.
//!
//! Calls `libc::kill` directly rather than spawning `/bin/kill`. Besides avoiding a subprocess
//! per click, this yields an `errno` we can branch on instead of string-matching stderr:
//! `EPERM` (not our process) and `ESRCH` (already gone) need very different handling and look
//! alike in text form.

use std::thread::sleep;
use std::time::{Duration, Instant};

/// How long to wait for a graceful SIGTERM exit before escalating to SIGKILL.
const GRACE_PERIOD: Duration = Duration::from_millis(500);
/// Liveness polling interval during the grace period.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillOutcome {
    /// Process is gone. Also covers "was already gone" — the caller wanted it dead and it is.
    Killed,
    /// `EPERM`: owned by another user. Caller should offer the `sudo` fallback.
    PermissionDenied,
    /// Anything else; carries a description for display.
    Failed(String),
}

/// Terminate `pid`, escalating from SIGTERM to SIGKILL only if needed.
///
/// SIGKILL cannot be trapped, so sending it first would deny dev servers their cleanup handlers
/// and can leave orphaned children. SIGTERM is tried first and escalated only if the process
/// outlives the grace period.
pub fn kill_process(pid: i32) -> KillOutcome {
    // Guard against signalling a process group (0 / negative pids) — `libc::kill` interprets
    // those as broadcast targets, which would be catastrophic here.
    if pid <= 0 {
        return KillOutcome::Failed(format!("refusing to signal pid {pid}"));
    }

    match send_signal(pid, libc::SIGTERM) {
        Ok(()) => {}
        Err(libc::ESRCH) => return KillOutcome::Killed, // already gone
        Err(libc::EPERM) => return KillOutcome::PermissionDenied,
        Err(errno) => return KillOutcome::Failed(describe_errno(errno)),
    }

    if wait_until_gone(pid, GRACE_PERIOD) {
        return KillOutcome::Killed;
    }

    // Survived SIGTERM — escalate.
    match send_signal(pid, libc::SIGKILL) {
        Ok(()) | Err(libc::ESRCH) => KillOutcome::Killed,
        Err(libc::EPERM) => KillOutcome::PermissionDenied,
        Err(errno) => KillOutcome::Failed(describe_errno(errno)),
    }
}

/// The command a user can paste to kill a process we lack permission for.
pub fn sudo_command_for(pid: i32) -> String {
    format!("sudo kill -9 {pid}")
}

/// Send `signal` to `pid`, mapping failure to a raw errno.
fn send_signal(pid: i32, signal: i32) -> Result<(), i32> {
    // SAFETY: `kill` is thread-safe and takes no pointers; `pid` is validated as > 0 by the
    // caller, so this cannot address a process group.
    if unsafe { libc::kill(pid, signal) } == 0 {
        return Ok(());
    }
    // errno must be read before any other libc call can overwrite it.
    Err(std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or(libc::EINVAL))
}

/// Poll until `pid` is gone or `timeout` elapses. Returns true if it is gone.
fn wait_until_gone(pid: i32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if !is_alive(pid) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        sleep(POLL_INTERVAL);
    }
}

/// Existence check via the null signal — performs permission and existence validation without
/// delivering anything.
///
/// Note: a zombie (exited but unreaped) still occupies a process table slot and reports as
/// alive. That only costs a redundant SIGKILL, which is a no-op on a zombie.
fn is_alive(pid: i32) -> bool {
    // SAFETY: signal 0 delivers nothing; same argument validity as `send_signal`.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    // EPERM means it exists but is not ours; only ESRCH means truly gone.
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn describe_errno(errno: i32) -> String {
    format!("{} (errno {errno})", std::io::Error::from_raw_os_error(errno))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn sudo_command_has_expected_form() {
        assert_eq!(sudo_command_for(12345), "sudo kill -9 12345");
    }

    #[test]
    fn terminates_an_owned_process() {
        let mut child = Command::new("sleep")
            .arg("30")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sleep");
        let pid = child.id() as i32;

        assert_eq!(kill_process(pid), KillOutcome::Killed);

        // Reap, so the pid is released rather than lingering as a zombie.
        let status = child.wait().expect("wait on child");
        assert!(!status.success(), "sleep should have been signalled, not exited cleanly");
    }

    #[test]
    fn killing_an_already_dead_process_succeeds() {
        let mut child = Command::new("true").spawn().expect("spawn true");
        let pid = child.id() as i32;
        child.wait().expect("reap child"); // fully gone and reaped

        // Target state (process not running) is already met, so this is success, not an error.
        assert_eq!(kill_process(pid), KillOutcome::Killed);
    }

    #[test]
    fn reports_permission_denied_for_launchd() {
        // pid 1 is launchd, owned by root — unkillable by an unprivileged user. Must surface as
        // PermissionDenied so the UI offers the sudo fallback, never as a generic failure.
        assert_eq!(kill_process(1), KillOutcome::PermissionDenied);
    }

    #[test]
    fn refuses_process_group_targets() {
        // pid 0 and negatives are process-group broadcasts in kill(2) and must never be sent.
        assert!(matches!(kill_process(0), KillOutcome::Failed(_)));
        assert!(matches!(kill_process(-1), KillOutcome::Failed(_)));
    }

    #[test]
    fn liveness_probe_tracks_process_state() {
        let mut child = Command::new("sleep").arg("30").spawn().expect("spawn sleep");
        let pid = child.id() as i32;
        assert!(is_alive(pid));

        child.kill().expect("kill child");
        child.wait().expect("reap child");
        assert!(!is_alive(pid), "reaped process must not report as alive");
    }
}
