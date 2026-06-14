//! SSH upload and remote command execution.

mod fake;
mod real;
mod transport;

pub use fake::FakeSshTransport;
pub use real::SystemSshTransport;
pub use transport::{RemoteCommandOutput, RemoteFile, SshTransport};
