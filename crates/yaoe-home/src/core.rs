use std::fmt;

use chrono::SecondsFormat;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    Success = 0,
    Cli = 2,
    Config = 3,
    State = 4,
    SingBox = 5,
    Cache = 6,
    Ssh = 8,
    Installer = 9,
    Cloudflare = 10,
    SrsFetch = 11,
    Upstream = 12,
    Gitee = 13,
    Internal = 14,
    HealthProbe = 15,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u8)
    }
}

#[derive(Debug, Error)]
pub enum YaoeError {
    #[error("CLI error: {0}")]
    Cli(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("state error: {0}")]
    State(String),
    #[error("local sing-box error: {0}")]
    SingBox(String),
    #[error("runtime cache error: {0}")]
    Cache(String),
    #[error("SSH error: {0}")]
    Ssh(String),
    #[error("installer error: {0}")]
    Installer(String),
    #[error("Cloudflare error: {0}")]
    Cloudflare(String),
    #[error("SRS fetch error: {0}")]
    SrsFetch(String),
    #[error("runtime upstream error: {0}")]
    Upstream(String),
    #[error("Gitee error: {0}")]
    Gitee(String),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("local active health probe error: {0}")]
    HealthProbe(String),
}

impl YaoeError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Cli(_) => ExitCode::Cli,
            Self::Config(_) => ExitCode::Config,
            Self::State(_) => ExitCode::State,
            Self::SingBox(_) => ExitCode::SingBox,
            Self::Cache(_) => ExitCode::Cache,
            Self::Ssh(_) => ExitCode::Ssh,
            Self::Installer(_) => ExitCode::Installer,
            Self::Cloudflare(_) => ExitCode::Cloudflare,
            Self::SrsFetch(_) => ExitCode::SrsFetch,
            Self::Upstream(_) => ExitCode::Upstream,
            Self::Gitee(_) => ExitCode::Gitee,
            Self::Internal(_) => ExitCode::Internal,
            Self::HealthProbe(_) => ExitCode::HealthProbe,
        }
    }
}

pub type YaoeResult<T> = Result<T, YaoeError>;

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex::encode(digest)
}

pub fn sha256_str(data: &str) -> String {
    sha256_hex(data.as_bytes())
}

pub fn digest_prefix(digest: &str) -> String {
    digest.chars().take(16).collect()
}

pub fn now_rfc3339_utc() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

impl LogLevel {
    pub fn token(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    fn sgr(self) -> &'static str {
        match self {
            Self::Info => "36",
            Self::Ok => "32",
            Self::Warn => "33",
            Self::Error => "31",
        }
    }
}

pub fn log_event(level: LogLevel, scope: &str, message: &str, fields: &[(&str, String)]) {
    let color = stderr_supports_color();
    let message = normalize_log_message(message);
    let level_token = if color {
        format!("\x1b[{}m{}\x1b[0m", level.sgr(), level.token())
    } else {
        level.token().to_string()
    };
    let scope_token = if color {
        format!("\x1b[1m{scope}\x1b[0m")
    } else {
        scope.to_string()
    };
    let rendered_fields = fields
        .iter()
        .map(|(key, value)| render_log_field(key, value))
        .collect::<Vec<_>>()
        .join(" ");
    if rendered_fields.is_empty() {
        eprintln!(
            "{} {} {} {}",
            now_rfc3339_utc(),
            level_token,
            scope_token,
            message.as_str()
        );
    } else {
        eprintln!(
            "{} {} {} {} {}",
            now_rfc3339_utc(),
            level_token,
            scope_token,
            message.as_str(),
            rendered_fields
        );
    }
}

pub fn log_value(value: &str) -> String {
    if !value.is_empty()
        && value.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '.' | '_' | ':' | '/' | '@' | '%' | '+' | '=' | ',' | '-')
        })
    {
        value.to_string()
    } else {
        serde_json::to_string(value).unwrap_or_else(|_| "\"<invalid>\"".to_string())
    }
}

fn render_log_field(key: &str, value: &str) -> String {
    if is_log_field_key(key) {
        format!("{key}={}", log_value(value))
    } else {
        format!("field={}", log_value(&format!("{key}={value}")))
    }
}

fn normalize_log_message(message: &str) -> String {
    let normalized = message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if normalized.is_empty() {
        "event".to_string()
    } else {
        normalized
    }
}

fn is_log_field_key(key: &str) -> bool {
    let mut chars = key.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

pub fn sanitize_external_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        if ch.is_ascii() && (ch == '\n' || ch == '\r' || ch == '\t' || !ch.is_control()) {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push(' ');
        }
    }
    out.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn stderr_supports_color() -> bool {
    std::io::IsTerminal::is_terminal(&std::io::stderr())
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("TERM").is_some_and(|term| term != "dumb")
}

#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn redacted(&self, kind: &str) -> String {
        redact_value(kind, self.expose())
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.redacted("secret"))
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.redacted("secret"))
    }
}

pub fn redact(_kind: &str, _value: &str) -> String {
    "<redacted>".to_string()
}

pub fn redact_value(_kind: &str, _value: &str) -> String {
    "<redacted>".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_is_fixed() {
        assert_eq!(redact("token", "secret"), "<redacted>");
    }

    #[test]
    fn sha256_is_lower_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn log_values_follow_unquoted_or_json_quoted_contract() {
        assert_eq!(log_value("abc-123_./:@%+=,"), "abc-123_./:@%+=,");
        assert_eq!(log_value(""), "\"\"");
        assert_eq!(log_value("has space"), "\"has space\"");
    }

    #[test]
    fn log_message_and_field_normalization_preserve_grammar() {
        assert_eq!(
            normalize_log_message("  Rendered\nConfig.  "),
            "rendered config"
        );
        assert_eq!(normalize_log_message(" \t "), "event");
        assert_eq!(render_log_field("ok_1", "true"), "ok_1=true");
        assert_eq!(render_log_field("Bad-Key", "value"), "field=Bad-Key=value");
    }

    #[test]
    fn external_text_sanitization_strips_ansi_and_icons() {
        let raw = "\u{1b}[33m▲ \u{1b}[1m[WARNING]\u{1b}[0m fetch failed\n🪵 logs";
        let cleaned = sanitize_external_text(raw);
        assert_eq!(cleaned, "[WARNING] fetch failed\nlogs");
        assert!(!cleaned.contains('\u{1b}'));
        assert!(!cleaned.contains('▲'));
        assert!(!cleaned.contains('🪵'));
    }
}
