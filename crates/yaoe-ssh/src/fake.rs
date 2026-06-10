use std::collections::HashMap;
use std::sync::Mutex;

use yaoe_home::{YaoeError, YaoeResult};

use crate::transport::{RemoteCommandOutput, SshTransport};

#[derive(Default)]
pub struct FakeSshTransport {
    pub files: Mutex<HashMap<String, String>>,
    pub commands: Mutex<Vec<String>>,
    pub outputs: Mutex<HashMap<String, String>>,
    pub raw_outputs: Mutex<HashMap<String, RemoteCommandOutput>>,
    pub errors: Mutex<HashMap<String, String>>,
}

impl FakeSshTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_output(&self, command_prefix: &str, output: &str) {
        self.outputs
            .lock()
            .unwrap()
            .insert(command_prefix.to_string(), output.to_string());
    }

    pub fn set_raw_output(&self, command_prefix: &str, output: RemoteCommandOutput) {
        self.raw_outputs
            .lock()
            .unwrap()
            .insert(command_prefix.to_string(), output);
    }

    pub fn set_status(&self, command_prefix: &str, status: i32, stderr: &str) {
        self.set_raw_output(
            command_prefix,
            RemoteCommandOutput {
                status,
                stdout: String::new(),
                stderr: stderr.to_string(),
            },
        );
    }

    pub fn set_error(&self, command_prefix: &str, error: &str) {
        self.errors
            .lock()
            .unwrap()
            .insert(command_prefix.to_string(), error.to_string());
    }

    fn health_probe_output(&self) -> String {
        format!(
            r#"yaoe_health.os=Linux
yaoe_health.arch=x86_64
yaoe_health.sing_box_version={}
yaoe_health.config_present=yes
yaoe_health.config_mode=600
yaoe_health.config_sha256=cfg
yaoe_health.unit_present=yes
yaoe_health.unit_mode=644
yaoe_health.unit_sha256=unit
yaoe_health.systemd_active=active
yaoe_health.port_listening=yes
yaoe_health.probe_elapsed_ms=3
"#,
            yaoe_home::sing_box_version_line()
        )
    }
}

impl SshTransport for FakeSshTransport {
    fn upload(
        &self,
        _destination: &str,
        local_path: &str,
        remote_path: &str,
        _key_path: &str,
    ) -> YaoeResult<()> {
        self.files
            .lock()
            .unwrap()
            .insert(remote_path.to_string(), local_path.to_string());
        Ok(())
    }

    fn run_as_root_raw(
        &self,
        _destination: &str,
        command: &str,
        _key_path: &str,
    ) -> YaoeResult<RemoteCommandOutput> {
        self.commands.lock().unwrap().push(command.to_string());
        let errors = self.errors.lock().unwrap();
        for (prefix, err) in errors.iter() {
            if command.contains(prefix) {
                return Err(YaoeError::Ssh(err.clone()));
            }
        }
        drop(errors);
        let raw_outputs = self.raw_outputs.lock().unwrap();
        for (prefix, out) in raw_outputs.iter() {
            if command.contains(prefix) {
                return Ok(out.clone());
            }
        }
        drop(raw_outputs);
        if command.contains("install.sh") {
            return Ok(success(String::new()));
        }
        if command.contains("YAOE_HEALTH_PROBE=1") {
            return Ok(success(self.health_probe_output()));
        }
        let outputs = self.outputs.lock().unwrap();
        for (prefix, out) in outputs.iter() {
            if command.contains(prefix) {
                return Ok(success(out.clone()));
            }
        }
        // Health check command responses.
        if command == "uname -s" {
            return Ok(success("Linux\n".into()));
        }
        if command == "uname -m" {
            return Ok(success("x86_64\n".into()));
        }
        if command.contains("sing-box version") {
            return Ok(success(format!("{}\n", yaoe_home::sing_box_version_line())));
        }
        if command.contains("test -f") && command.contains("&& echo PRESENT") {
            return Ok(success("PRESENT\n".into()));
        }
        if command.contains("systemctl is-enabled") {
            return Ok(success("enabled\n".into()));
        }
        if command.contains("systemctl is-active") || command.contains("systemctl restart") {
            return Ok(success("active\n".into()));
        }
        if command.contains("ss -ltn") || command.contains("ss -lntp") {
            return Ok(success("LISTEN 0 128 0.0.0.0:443\n".into()));
        }
        Ok(success(String::new()))
    }

    fn read_file(
        &self,
        _destination: &str,
        remote_path: &str,
        _key_path: &str,
    ) -> YaoeResult<String> {
        Ok(self
            .files
            .lock()
            .unwrap()
            .get(remote_path)
            .cloned()
            .unwrap_or_default())
    }
}

fn success(stdout: String) -> RemoteCommandOutput {
    RemoteCommandOutput {
        status: 0,
        stdout,
        stderr: String::new(),
    }
}
