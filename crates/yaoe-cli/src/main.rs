use std::process::ExitCode;
use std::{fs, path::Path};

use clap::{Parser, Subcommand};
use yaoe_controller::{
    RuntimeDeps, cmd_apply, cmd_check, cmd_client, cmd_health, cmd_init, cmd_publish_bootstrap,
    cmd_publish_config, cmd_publish_delivery, cmd_publish_runtime, cmd_rotate_config_key,
    cmd_rotate_reality_keypair, cmd_rotate_vless_uuid, cmd_status,
};
use yaoe_home::{ExitCode as Yc, LogLevel, YaoeError, log_event, sanitize_external_text};

#[derive(Parser)]
#[command(name = "yaoe", version, about = "YAOE egress controller")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Check,
    Client,
    Rotate {
        #[command(subcommand)]
        target: RotateCommands,
    },
    Apply {
        server: Option<String>,
    },
    Publish {
        #[command(subcommand)]
        target: PublishCommands,
    },
    Status {
        server: Option<String>,
    },
    Health {
        server: Option<String>,
    },
}

#[derive(Subcommand)]
enum PublishCommands {
    Bootstrap,
    Runtime,
    Config,
    Delivery,
}

#[derive(Subcommand)]
enum RotateCommands {
    ConfigKey,
    VlessUuid,
    RealityKeypair,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::from(Yc::Success.as_i32() as u8),
        Err(err) => {
            log_event(
                LogLevel::Error,
                "yaoe",
                "error",
                &[("message", redact_error_text(&err.to_string()))],
            );
            ExitCode::from(err.exit_code().as_i32() as u8)
        }
    }
}

fn run() -> Result<(), YaoeError> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => {
            let deps = RuntimeDeps::production_ssh_only()?;
            cmd_init(None, &deps)?;
        }
        Commands::Check => cmd_check(None)?,
        Commands::Client => cmd_client(None)?,
        Commands::Rotate { target } => match target {
            RotateCommands::ConfigKey => cmd_rotate_config_key(None)?,
            RotateCommands::VlessUuid => cmd_rotate_vless_uuid(None)?,
            RotateCommands::RealityKeypair => {
                let deps = RuntimeDeps::production_ssh_only()?;
                cmd_rotate_reality_keypair(None, &deps)?;
            }
        },
        Commands::Apply { server } => {
            let deps = load_deps()?;
            cmd_apply(None, server.as_deref(), &deps)?;
        }
        Commands::Publish { target } => {
            let deps = load_deps()?;
            match target {
                PublishCommands::Bootstrap => cmd_publish_bootstrap(None, &deps)?,
                PublishCommands::Runtime => cmd_publish_runtime(None, &deps)?,
                PublishCommands::Config => cmd_publish_config(None, &deps)?,
                PublishCommands::Delivery => cmd_publish_delivery(None, &deps)?,
            }
        }
        Commands::Status { server } => {
            let deps = RuntimeDeps::production_ssh_only()?;
            cmd_status(None, server.as_deref(), &deps)?;
        }
        Commands::Health { server } => {
            let deps = RuntimeDeps::production_ssh_only()?;
            cmd_health(None, server.as_deref(), &deps)?;
        }
    }
    Ok(())
}

fn load_deps() -> Result<RuntimeDeps, YaoeError> {
    let paths = yaoe_home::resolve_home(None);
    let config = yaoe_config::load_and_validate(&paths.config)?;
    RuntimeDeps::production(&config)
}

fn redact_error_text(text: &str) -> String {
    let mut out = text.to_string();
    for secret in local_sensitive_values() {
        if !secret.is_empty() {
            out = out.replace(&secret, "<redacted>");
        }
    }
    for pattern in [
        r"cf[a-zA-Z0-9_-]{8,}",
        r"[A-Za-z0-9_-]{128}",
        r"[A-Za-z0-9_-]{43}",
        r"[0-9a-f]{16}",
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
    ] {
        if let Ok(re) = regex::Regex::new(pattern) {
            out = re.replace_all(&out, "<redacted>").to_string();
        }
    }
    sanitize_external_text(&out)
}

fn local_sensitive_values() -> Vec<String> {
    let paths = yaoe_home::resolve_home(None);
    let mut values = Vec::new();
    if let Some(value) = read_config_value(&paths.config, &["cloudflare", "token"]) {
        values.push(value);
    }
    if let Some(value) = read_config_value(&paths.config, &["gitee", "token"]) {
        values.push(value);
    }
    for path in [
        ["credential", "vless_uuid"],
        ["credential", "config_key"],
        ["credential", "reality_private_key"],
        ["credential", "reality_short_id"],
    ] {
        if let Some(value) = read_config_value(&paths.config, &path) {
            values.push(value);
        }
    }
    values
}

fn read_config_value(path: &Path, keys: &[&str]) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let value = text.parse::<toml::Value>().ok()?;
    let mut current = &value;
    for key in keys {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_error_redaction_removes_terminal_sequences_and_icons() {
        let text = "\u{1b}[31m✘\u{1b}[0m cfabcdefghijklmnopqrstuvwxyz123456 fetch failed 🪵";
        let redacted = redact_error_text(text);
        assert!(!redacted.contains('\u{1b}'));
        assert!(!redacted.contains('✘'));
        assert!(!redacted.contains('🪵'));
        assert!(!redacted.contains("cfabcdefghijklmnopqrstuvwxyz123456"));
        assert!(redacted.contains("<redacted>"));
        assert!(redacted.contains("fetch failed"));
    }
}
