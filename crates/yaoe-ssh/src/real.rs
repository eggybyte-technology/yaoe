use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;
use yaoe_home::{YaoeError, YaoeResult, redact};

use crate::transport::{RemoteCommandOutput, SshTransport};

pub struct SystemSshTransport;

const SCP_UPLOAD_TIMEOUT: Duration = Duration::from_secs(45);
const SSH_COMMAND_TIMEOUT: Duration = Duration::from_secs(45);
const SSH_INSTALL_TIMEOUT: Duration = Duration::from_secs(300);
const SSH_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const SSH_STREAM_UPLOAD_TIMEOUT: Duration = Duration::from_secs(90);

impl Default for SystemSshTransport {
    fn default() -> Self {
        Self
    }
}

impl SystemSshTransport {
    pub fn new() -> Self {
        Self
    }

    fn expand_key(key_path: &str) -> YaoeResult<String> {
        if let Some(rest) = key_path.strip_prefix("~/") {
            let home = std::env::var("HOME")
                .map_err(|_| YaoeError::Ssh("HOME not set for ~ expansion".into()))?;
            Ok(format!("{home}/{rest}"))
        } else {
            Ok(key_path.to_string())
        }
    }

    fn base_ssh(destination: &str, key: &str) -> Vec<String> {
        vec![
            "-i".into(),
            key.to_string(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "IdentitiesOnly=yes".into(),
            "-o".into(),
            "StrictHostKeyChecking=accept-new".into(),
            "-o".into(),
            "IPQoS=none".into(),
            destination.into(),
        ]
    }
}

impl SshTransport for SystemSshTransport {
    fn upload(
        &self,
        destination: &str,
        local_path: &str,
        remote_path: &str,
        key_path: &str,
    ) -> YaoeResult<()> {
        let key = Self::expand_key(key_path)?;
        let mut command = Command::new("scp");
        command
            .arg("-i")
            .arg(&key)
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("IdentitiesOnly=yes")
            .arg("-o")
            .arg("StrictHostKeyChecking=accept-new")
            .arg("-o")
            .arg("IPQoS=none")
            .arg(local_path)
            .arg(format!("{destination}:{remote_path}"));
        match output_with_timeout(command, SCP_UPLOAD_TIMEOUT, "scp upload") {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let scp_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
                ssh_stream_upload(destination, local_path, remote_path, &key).map_err(|err| {
                    YaoeError::Ssh(format!(
                        "scp upload failed for {}: {}; ssh stream upload failed: {err}",
                        redact("path", local_path),
                        scp_error
                    ))
                })
            }
            Err(err) => {
                let scp_error = err.to_string();
                ssh_stream_upload(destination, local_path, remote_path, &key).map_err(|err| {
                    YaoeError::Ssh(format!(
                        "scp upload failed for {}: {}; ssh stream upload failed: {err}",
                        redact("path", local_path),
                        scp_error
                    ))
                })
            }
        }
    }

    fn run_as_root_raw(
        &self,
        destination: &str,
        command: &str,
        key_path: &str,
    ) -> YaoeResult<RemoteCommandOutput> {
        let key = Self::expand_key(key_path)?;
        let mut child = Command::new("ssh");
        child.args(Self::base_ssh(destination, &key)).arg(command);
        let output = output_with_timeout(child, timeout_for_remote_command(command), "ssh")?;
        Ok(RemoteCommandOutput {
            status: output.status.code().unwrap_or(255),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn read_file(
        &self,
        destination: &str,
        remote_path: &str,
        key_path: &str,
    ) -> YaoeResult<String> {
        self.run_as_root(
            destination,
            &format!("cat {}", shell_escape(remote_path)),
            key_path,
        )
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn timeout_for_remote_command(command: &str) -> Duration {
    if command.contains("install.sh") {
        SSH_INSTALL_TIMEOUT
    } else if command.starts_with("systemctl is-active ") {
        SSH_PROBE_TIMEOUT
    } else {
        SSH_COMMAND_TIMEOUT
    }
}

fn ssh_stream_upload(
    destination: &str,
    local_path: &str,
    remote_path: &str,
    key: &str,
) -> YaoeResult<()> {
    let remote_command = format!("cat > {}", shell_escape(remote_path));
    let script = format!(
        "exec ssh -i {} -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new -o IPQoS=none {} {} < {}",
        shell_escape(key),
        shell_escape(destination),
        shell_escape(&remote_command),
        shell_escape(local_path)
    );
    let mut command = Command::new("sh");
    command.arg("-c").arg(script);
    let output = output_with_timeout(command, SSH_STREAM_UPLOAD_TIMEOUT, "ssh stream upload")?;
    if !output.status.success() {
        return Err(YaoeError::Ssh(format!(
            "remote cat upload exited with status {}: {}",
            output.status.code().unwrap_or(255),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

fn output_with_timeout(
    mut command: Command,
    timeout: Duration,
    description: &str,
) -> YaoeResult<Output> {
    command.process_group(0);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| YaoeError::Ssh(format!("{description} failed: {e}")))?;
    match child
        .wait_timeout(timeout)
        .map_err(|e| YaoeError::Ssh(format!("wait for {description}: {e}")))?
    {
        Some(_) => child
            .wait_with_output()
            .map_err(|e| YaoeError::Ssh(format!("collect {description} output: {e}"))),
        None => {
            kill_process_group(child.id());
            let _ = child.kill();
            let output = child.wait_with_output().map_err(|e| {
                YaoeError::Ssh(format!("collect timed-out {description} output: {e}"))
            })?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.trim();
            if detail.is_empty() {
                Err(YaoeError::Ssh(format!(
                    "{description} timed out after {}s",
                    timeout.as_secs()
                )))
            } else {
                Err(YaoeError::Ssh(format!(
                    "{description} timed out after {}s: {detail}",
                    timeout.as_secs()
                )))
            }
        }
    }
}

fn kill_process_group(pid: u32) {
    let pgid = -(pid as i32);
    // SAFETY: kill(2) with a negative pid sends the signal to that process group.
    unsafe {
        libc::kill(pgid, libc::SIGKILL);
    }
}
