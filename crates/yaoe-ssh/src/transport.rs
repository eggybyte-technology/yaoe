use yaoe_home::{YaoeError, YaoeResult};

#[derive(Debug, Clone)]
pub struct RemoteFile {
    pub path: String,
    pub content: Vec<u8>,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait SshTransport: Send + Sync {
    fn upload(
        &self,
        destination: &str,
        local_path: &str,
        remote_path: &str,
        key_path: &str,
    ) -> YaoeResult<()>;

    fn run_as_root_raw(
        &self,
        destination: &str,
        command: &str,
        key_path: &str,
    ) -> YaoeResult<RemoteCommandOutput>;

    fn run_as_root(&self, destination: &str, command: &str, key_path: &str) -> YaoeResult<String> {
        let output = self.run_as_root_raw(destination, command, key_path)?;
        if output.status != 0 {
            return Err(YaoeError::Ssh(format!(
                "remote command failed with status {}: {}",
                output.status,
                output.stderr.trim()
            )));
        }
        Ok(output.stdout)
    }

    fn read_file(&self, destination: &str, remote_path: &str, key_path: &str)
    -> YaoeResult<String>;
}
