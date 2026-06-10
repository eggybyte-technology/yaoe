//! Home layout, atomic writes, generated state, and runtime cache paths.

mod constants;
mod core;
mod home;
mod paths;

pub use constants::*;
pub use core::{
    ExitCode, LogLevel, Secret, YaoeError, YaoeResult, digest_prefix, log_event, log_value,
    now_rfc3339_utc, redact, redact_value, sanitize_external_text, sha256_hex, sha256_str,
};
pub use home::{
    atomic_rename, atomic_write, ensure_home, init_home, read_secret_file, require_regular_file,
    resolve_home, validate_home_layout, write_secret_file,
};
pub use paths::DEFAULT_HOME;
pub use paths::HomePaths;
