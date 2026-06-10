//! Local process adapters for validation, key generation, and active probes.

use std::net::TcpStream;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use yaoe_config::derive_reality_public_key;
use yaoe_home::{
    HEALTH_PROBE_BIND_HOST, HEALTH_PROBE_EXPECTED_STATUS, HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS,
    HEALTH_PROBE_STARTUP_TIMEOUT_SECONDS, HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS, HEALTH_PROBE_URL,
    MIHOMO_VALIDATION_VERSION, REMOTE_JOURNAL_TAIL_LINES, SING_BOX_VERSION, YaoeError, YaoeResult,
    sing_box_version_line,
};

use crate::logging::{info, ok, tail_lines};

pub trait LocalSingBox: Send + Sync {
    fn require_version(&self) -> YaoeResult<()>;
    fn check_config(&self, path: &Path) -> YaoeResult<()>;
    fn run_health_probe(&self, config_path: &Path, probe_port: u16, server: &str)
    -> ProbeRunResult;
}

pub struct SystemLocalSingBox;

pub trait LocalMihomo: Send + Sync {
    fn require_version(&self) -> YaoeResult<()>;
    fn check_config(&self, path: &Path) -> YaoeResult<()>;
}

pub struct SystemLocalMihomo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSuccess {
    pub status: u16,
    pub elapsed_ms: u128,
    pub pid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeFailure {
    pub stage: &'static str,
    pub curl_status: Option<String>,
    pub curl_exit: Option<i32>,
    pub stderr_tail: String,
    pub detail: String,
}

pub type ProbeRunResult = Result<ProbeSuccess, ProbeFailure>;

impl LocalSingBox for SystemLocalSingBox {
    fn require_version(&self) -> YaoeResult<()> {
        let output = Command::new("sing-box")
            .arg("version")
            .output()
            .map_err(|e| YaoeError::SingBox(format!("run sing-box version: {e}")))?;
        let first_line = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        if !output.status.success() || first_line != sing_box_version_line() {
            return Err(YaoeError::SingBox(format!(
                "sing-box from PATH must report version {SING_BOX_VERSION}"
            )));
        }
        Ok(())
    }

    fn check_config(&self, path: &Path) -> YaoeResult<()> {
        let output = Command::new("sing-box")
            .arg("check")
            .arg("-c")
            .arg(path)
            .output()
            .map_err(|e| YaoeError::SingBox(format!("run sing-box check: {e}")))?;
        if !output.status.success() {
            return Err(YaoeError::SingBox(format!(
                "sing-box check failed for {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    fn run_health_probe(
        &self,
        config_path: &Path,
        probe_port: u16,
        server: &str,
    ) -> ProbeRunResult {
        let start = Instant::now();
        let mut child = match Command::new("sing-box")
            .args(["run", "-c"])
            .arg(config_path)
            .process_group(0)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                return Err(ProbeFailure {
                    stage: "spawn",
                    curl_status: None,
                    curl_exit: None,
                    stderr_tail: String::new(),
                    detail: format!("start sing-box probe: {err}"),
                });
            }
        };

        let startup_deadline =
            Instant::now() + Duration::from_secs(HEALTH_PROBE_STARTUP_TIMEOUT_SECONDS);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stderr_tail = stop_probe_child(child);
                    return Err(ProbeFailure {
                        stage: "startup",
                        curl_status: None,
                        curl_exit: status.code(),
                        stderr_tail,
                        detail: "sing-box probe exited before mixed inbound was reachable".into(),
                    });
                }
                Ok(None) => {}
                Err(err) => {
                    let stderr_tail = stop_probe_child(child);
                    return Err(ProbeFailure {
                        stage: "startup",
                        curl_status: None,
                        curl_exit: None,
                        stderr_tail,
                        detail: format!("check sing-box probe process: {err}"),
                    });
                }
            }
            if TcpStream::connect((HEALTH_PROBE_BIND_HOST, probe_port)).is_ok() {
                ok(
                    &format!("health.probe:{server}"),
                    "mixed inbound ready",
                    &[("bind", format!("{HEALTH_PROBE_BIND_HOST}:{probe_port}"))],
                );
                break;
            }
            if Instant::now() >= startup_deadline {
                let stderr_tail = stop_probe_child(child);
                return Err(ProbeFailure {
                    stage: "startup",
                    curl_status: None,
                    curl_exit: None,
                    stderr_tail,
                    detail: "mixed inbound did not become reachable before timeout".into(),
                });
            }
            thread::sleep(Duration::from_millis(50));
        }

        info(
            &format!("health.probe:{server}"),
            "running curl probe",
            &[
                (
                    "proxy",
                    format!("socks5://{HEALTH_PROBE_BIND_HOST}:{probe_port}"),
                ),
                ("resolve", "remote_hostname_ipv4".to_string()),
                ("url", HEALTH_PROBE_URL.to_string()),
            ],
        );
        let curl_args = health_probe_curl_args(probe_port);
        let output = Command::new("curl").args(&curl_args).output();
        let pid = child.id();
        let output = match output {
            Ok(output) => output,
            Err(err) => {
                let stderr_tail = stop_probe_child(child);
                return Err(ProbeFailure {
                    stage: "request",
                    curl_status: None,
                    curl_exit: None,
                    stderr_tail,
                    detail: format!("run curl health request: {err}"),
                });
            }
        };
        let status_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !output.status.success() || status_text != HEALTH_PROBE_EXPECTED_STATUS.to_string() {
            let stderr_tail = stop_probe_child(child);
            return Err(ProbeFailure {
                stage: "request",
                curl_status: (!status_text.is_empty()).then_some(status_text),
                curl_exit: output.status.code(),
                stderr_tail,
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        let _ = stop_probe_child(child);
        Ok(ProbeSuccess {
            status: HEALTH_PROBE_EXPECTED_STATUS,
            elapsed_ms: start.elapsed().as_millis(),
            pid,
        })
    }
}

impl LocalMihomo for SystemLocalMihomo {
    fn require_version(&self) -> YaoeResult<()> {
        let output = Command::new("mihomo")
            .arg("-v")
            .output()
            .map_err(|e| YaoeError::SingBox(format!("run mihomo -v: {e}")))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() || !stdout.contains(MIHOMO_VALIDATION_VERSION) {
            return Err(YaoeError::SingBox(format!(
                "mihomo from PATH must report version {MIHOMO_VALIDATION_VERSION}"
            )));
        }
        Ok(())
    }

    fn check_config(&self, path: &Path) -> YaoeResult<()> {
        let output = Command::new("mihomo")
            .arg("-t")
            .arg("-f")
            .arg(path)
            .output()
            .map_err(|e| YaoeError::SingBox(format!("run mihomo config check: {e}")))?;
        if !output.status.success() {
            return Err(YaoeError::SingBox(format!(
                "mihomo check failed for {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }
}

pub(crate) fn health_probe_curl_args(probe_port: u16) -> Vec<String> {
    vec![
        "-fsS".to_string(),
        "--ipv4".to_string(),
        "--socks5-hostname".to_string(),
        format!("{HEALTH_PROBE_BIND_HOST}:{probe_port}"),
        "--connect-timeout".to_string(),
        HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS.to_string(),
        "--max-time".to_string(),
        HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS.to_string(),
        "--output".to_string(),
        "/dev/null".to_string(),
        "--write-out".to_string(),
        "%{http_code}".to_string(),
        HEALTH_PROBE_URL.to_string(),
    ]
}

pub trait RealityKeypairGenerator: Send + Sync {
    fn generate(&self) -> YaoeResult<(String, String)>;
}

pub struct SystemRealityKeypairGenerator;

impl RealityKeypairGenerator for SystemRealityKeypairGenerator {
    fn generate(&self) -> YaoeResult<(String, String)> {
        let output = Command::new("sing-box")
            .args(["generate", "reality-keypair"])
            .output()
            .map_err(|e| {
                YaoeError::Config(format!("run sing-box generate reality-keypair: {e}"))
            })?;
        if !output.status.success() {
            return Err(YaoeError::Config(format!(
                "sing-box generate reality-keypair failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        parse_reality_keypair_output(&String::from_utf8_lossy(&output.stdout))
    }
}

pub trait PublicConfigFetcher: Send + Sync {
    fn fetch_ok(&self, url: &str) -> YaoeResult<Option<Vec<u8>>>;
}

pub struct ReqwestPublicConfigFetcher {
    client: Client,
}

impl ReqwestPublicConfigFetcher {
    pub fn new() -> YaoeResult<Self> {
        let client = Client::builder()
            .user_agent(format!(
                "yaoe/{}",
                yaoe_home::YAOE_PRODUCT_REVISION.trim_start_matches('v')
            ))
            .build()
            .map_err(|e| YaoeError::Cloudflare(format!("http client: {e}")))?;
        Ok(Self { client })
    }
}

impl PublicConfigFetcher for ReqwestPublicConfigFetcher {
    fn fetch_ok(&self, url: &str) -> YaoeResult<Option<Vec<u8>>> {
        let resp = match self.client.get(url).send() {
            Ok(resp) => resp,
            Err(_) => return Ok(None),
        };
        if resp.status() != reqwest::StatusCode::OK {
            return Ok(None);
        }
        let bytes = resp
            .bytes()
            .map_err(|e| YaoeError::Cloudflare(format!("read public config: {e}")))?;
        Ok(Some(bytes.to_vec()))
    }
}

fn parse_reality_keypair_output(output: &str) -> YaoeResult<(String, String)> {
    let mut private = None;
    let mut public = None;
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("private") {
            private = line.split_whitespace().last().map(str::to_string);
        } else if lower.contains("public") {
            public = line.split_whitespace().last().map(str::to_string);
        }
    }
    let private = private
        .ok_or_else(|| YaoeError::Config("sing-box output missing Reality private key".into()))?;
    let public = public
        .ok_or_else(|| YaoeError::Config("sing-box output missing Reality public key".into()))?;
    let derived = derive_reality_public_key(&private)?;
    if derived != public {
        return Err(YaoeError::Config(
            "derived Reality public key does not match sing-box output".into(),
        ));
    }
    Ok((private, public))
}

fn stop_probe_child(mut child: Child) -> String {
    kill_process_group(child.id());
    let _ = child.kill();
    match child.wait_with_output() {
        Ok(output) => tail_lines(
            &String::from_utf8_lossy(&output.stderr),
            REMOTE_JOURNAL_TAIL_LINES,
        ),
        Err(err) => format!("collect probe stderr failed: {err}"),
    }
}

fn kill_process_group(pid: u32) {
    let pgid = -(pid as i32);
    // SAFETY: kill(2) with a negative pid sends the signal to the process group.
    unsafe {
        libc::kill(pgid, libc::SIGTERM);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_probe_curl_uses_socks5_remote_hostname_resolution() {
        let args = health_probe_curl_args(2080);
        assert!(args.iter().any(|arg| arg == "--ipv4"));
        assert_eq!(
            args.windows(2)
                .find(|pair| pair[0] == "--socks5-hostname")
                .map(|pair| pair[1].as_str()),
            Some("127.0.0.1:2080")
        );
        assert!(!args.iter().any(|arg| arg == "--socks5"));
        assert!(!args.iter().any(|arg| arg == "--proxy"));
        assert!(args.iter().any(|arg| arg == HEALTH_PROBE_URL));
    }
}
