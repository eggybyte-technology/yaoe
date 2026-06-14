use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use wait_timeout::ChildExt;
use yaoe_home::{
    LogLevel, R2_CONFIG_CACHE_CONTROL, R2_CUSTOM_DOMAIN_MIN_TLS, YaoeError, YaoeResult,
    config_variant, log_event as home_log_event, sanitize_external_text,
};

const CLOUDFLARE_HTTP_ATTEMPTS: usize = 4;
const WRANGLER_ATTEMPTS: usize = 3;

pub trait CloudflareZoneResolver: Send + Sync {
    fn resolve_zone_id(&self, delivery_domain: &str) -> YaoeResult<String>;
}

pub struct CloudflareClient {
    http: Client,
    token: String,
}

impl CloudflareClient {
    pub fn new(token: impl Into<String>) -> YaoeResult<Self> {
        let http = Client::builder()
            .user_agent(format!(
                "yaoe/{}",
                yaoe_home::YAOE_PRODUCT_REVISION.trim_start_matches('v')
            ))
            .build()
            .map_err(|e| YaoeError::Cloudflare(format!("http client: {e}")))?;
        Ok(Self {
            http,
            token: token.into(),
        })
    }
}

impl CloudflareZoneResolver for CloudflareClient {
    fn resolve_zone_id(&self, delivery_domain: &str) -> YaoeResult<String> {
        let labels: Vec<&str> = delivery_domain.split('.').collect();
        if labels.len() < 3 {
            return Err(YaoeError::Cloudflare(
                "delivery domain must have at least three labels".into(),
            ));
        }
        for idx in 1..labels.len() - 1 {
            let candidate = labels[idx..].join(".");
            let url = format!("https://api.cloudflare.com/client/v4/zones?name={candidate}");
            let resp = self.get_with_retry(&url, "zones", &candidate)?;
            let status = resp.status();
            let text = resp
                .text()
                .map_err(|e| YaoeError::Cloudflare(format!("zone response: {e}")))?;
            if !status.is_success() {
                let text = text.replace(&self.token, "<redacted>");
                return Err(YaoeError::Cloudflare(format!(
                    "zone lookup {candidate} returned {status}: {text}"
                )));
            }
            let envelope: CfEnvelope<Vec<Zone>> = serde_json::from_str(&text)
                .map_err(|e| YaoeError::Cloudflare(format!("parse zone response: {e}")))?;
            if !envelope.success {
                return Err(YaoeError::Cloudflare(
                    "Cloudflare zone lookup was not successful".into(),
                ));
            }
            let matches: Vec<_> = envelope
                .result
                .unwrap_or_default()
                .into_iter()
                .filter(|z| z.name == candidate && z.status == "active")
                .collect();
            if matches.len() > 1 {
                return Err(YaoeError::Cloudflare(format!(
                    "multiple active zones named {candidate}"
                )));
            }
            if let Some(zone) = matches.into_iter().next() {
                if idx != 1 {
                    return Err(YaoeError::Cloudflare(format!(
                        "{delivery_domain} must be exactly one label below {candidate}"
                    )));
                }
                return Ok(zone.id);
            }
        }
        Err(YaoeError::Cloudflare(format!(
            "no accessible active Cloudflare zone owns {delivery_domain}"
        )))
    }
}

impl CloudflareClient {
    fn get_with_retry(
        &self,
        url: &str,
        operation: &str,
        target: &str,
    ) -> YaoeResult<reqwest::blocking::Response> {
        let mut last_error = None;
        for attempt in 1..=CLOUDFLARE_HTTP_ATTEMPTS {
            match self.http.get(url).bearer_auth(&self.token).send() {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let err_text =
                        sanitize_external_text(&err.to_string()).replace(&self.token, "<redacted>");
                    home_log_event(
                        LogLevel::Warn,
                        "cloudflare.http",
                        "retrying request",
                        &[
                            ("operation", operation.to_string()),
                            ("target", target.to_string()),
                            ("attempt", format!("{attempt}/{CLOUDFLARE_HTTP_ATTEMPTS}")),
                            ("error", err_text.clone()),
                        ],
                    );
                    last_error = Some(err_text);
                    if attempt != CLOUDFLARE_HTTP_ATTEMPTS {
                        std::thread::sleep(Duration::from_secs((attempt.min(4) * 2) as u64));
                    }
                }
            }
        }
        Err(YaoeError::Cloudflare(format!(
            "zone lookup {target}: {}",
            last_error.unwrap_or_else(|| "request failed".to_string())
        )))
    }
}

#[derive(Debug, Deserialize)]
struct CfEnvelope<T> {
    success: bool,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct Zone {
    id: String,
    name: String,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainState {
    pub min_tls: Option<String>,
}

pub trait R2Wrangler: Send + Sync {
    fn bucket_exists(&self, account_id: &str, token: &str, bucket: &str) -> YaoeResult<bool>;
    fn create_bucket(&self, account_id: &str, token: &str, bucket: &str) -> YaoeResult<()>;
    fn domain_state(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        domain: &str,
    ) -> YaoeResult<Option<DomainState>>;
    fn add_domain(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        domain: &str,
        zone_id: &str,
    ) -> YaoeResult<()>;
    fn update_domain_tls(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        domain: &str,
    ) -> YaoeResult<()>;
    fn put_object(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        object_key: &str,
        file: &Path,
        content_type: &str,
    ) -> YaoeResult<()>;
}

pub struct SystemR2Wrangler;

const WRANGLER_TIMEOUT: Duration = Duration::from_secs(300);

impl R2Wrangler for SystemR2Wrangler {
    fn bucket_exists(&self, account_id: &str, token: &str, bucket: &str) -> YaoeResult<bool> {
        let output = wrangler(account_id, token, &["r2", "bucket", "list"])?;
        Ok(bucket_list_contains(&output, bucket))
    }

    fn create_bucket(&self, account_id: &str, token: &str, bucket: &str) -> YaoeResult<()> {
        wrangler(account_id, token, &["r2", "bucket", "create", bucket]).map(|_| ())
    }

    fn domain_state(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        domain: &str,
    ) -> YaoeResult<Option<DomainState>> {
        match wrangler(
            account_id,
            token,
            &["r2", "bucket", "domain", "get", bucket, "--domain", domain],
        ) {
            Ok(output) => {
                let min_tls = parse_min_tls(&output);
                Ok(Some(DomainState { min_tls }))
            }
            Err(YaoeError::Cloudflare(err)) if looks_like_missing_domain(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn add_domain(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        domain: &str,
        zone_id: &str,
    ) -> YaoeResult<()> {
        wrangler(
            account_id,
            token,
            &[
                "r2",
                "bucket",
                "domain",
                "add",
                bucket,
                "--domain",
                domain,
                "--zone-id",
                zone_id,
                "--min-tls",
                R2_CUSTOM_DOMAIN_MIN_TLS,
                "--force",
            ],
        )
        .map(|_| ())
    }

    fn update_domain_tls(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        domain: &str,
    ) -> YaoeResult<()> {
        wrangler(
            account_id,
            token,
            &[
                "r2",
                "bucket",
                "domain",
                "update",
                bucket,
                "--domain",
                domain,
                "--min-tls",
                R2_CUSTOM_DOMAIN_MIN_TLS,
            ],
        )
        .map(|_| ())
    }

    fn put_object(
        &self,
        account_id: &str,
        token: &str,
        bucket: &str,
        object_key: &str,
        file: &Path,
        content_type: &str,
    ) -> YaoeResult<()> {
        let target = format!("{bucket}/{object_key}");
        let file_s = file.to_str().ok_or_else(|| {
            YaoeError::Cloudflare(format!("non-UTF8 file path {}", file.display()))
        })?;
        wrangler(
            account_id,
            token,
            &[
                "r2",
                "object",
                "put",
                &target,
                "--file",
                file_s,
                "--content-type",
                content_type,
                "--cache-control",
                R2_CONFIG_CACHE_CONTROL,
                "--remote",
                "--force",
            ],
        )
        .map(|_| ())
    }
}

fn wrangler(account_id: &str, token: &str, args: &[&str]) -> YaoeResult<String> {
    let operation = sanitized_wrangler_operation(args);
    let mut last_error = None;
    for attempt in 1..=WRANGLER_ATTEMPTS {
        log_event(
            "cloudflare",
            "wrangler",
            &[
                ("operation", operation.clone()),
                ("action", "start".to_string()),
                ("attempt", format!("{attempt}/{WRANGLER_ATTEMPTS}")),
            ],
        );
        let output = run_wrangler_once(account_id, token, args, &operation)?;
        if output.status.success() {
            log_event(
                "cloudflare",
                "wrangler",
                &[
                    ("operation", operation),
                    ("action", "finish".to_string()),
                    ("attempt", format!("{attempt}/{WRANGLER_ATTEMPTS}")),
                ],
            );
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        let stderr = sanitize_wrangler_stderr(&output.stderr, token, account_id);
        let message = format!("wrangler {operation} failed: {}", stderr.trim());
        if !is_retryable_wrangler_failure(&stderr) || attempt == WRANGLER_ATTEMPTS {
            return Err(YaoeError::Cloudflare(message));
        }
        home_log_event(
            LogLevel::Warn,
            "cloudflare.wrangler",
            "retrying command",
            &[
                ("operation", operation.clone()),
                ("attempt", format!("{attempt}/{WRANGLER_ATTEMPTS}")),
                ("error", stderr),
            ],
        );
        last_error = Some(message);
        std::thread::sleep(Duration::from_secs((attempt.min(4) * 2) as u64));
    }
    Err(YaoeError::Cloudflare(
        last_error.unwrap_or_else(|| format!("wrangler {operation} failed")),
    ))
}

fn run_wrangler_once(
    account_id: &str,
    token: &str,
    args: &[&str],
    operation: &str,
) -> YaoeResult<Output> {
    let mut child = Command::new("wrangler")
        .env("CLOUDFLARE_ACCOUNT_ID", account_id)
        .env("CLOUDFLARE_API_TOKEN", token)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| YaoeError::Cloudflare(format!("run wrangler: {e}")))?;
    match child
        .wait_timeout(WRANGLER_TIMEOUT)
        .map_err(|e| YaoeError::Cloudflare(format!("wait for wrangler {operation}: {e}")))?
    {
        Some(_) => child
            .wait_with_output()
            .map_err(|e| YaoeError::Cloudflare(format!("collect wrangler {operation}: {e}"))),
        None => {
            let _ = child.kill();
            let output = child.wait_with_output().map_err(|e| {
                YaoeError::Cloudflare(format!("collect timed-out wrangler {operation}: {e}"))
            })?;
            let stderr = sanitize_wrangler_stderr(&output.stderr, token, account_id);
            Err(YaoeError::Cloudflare(format!(
                "wrangler {operation} timed out after {}s: {}",
                WRANGLER_TIMEOUT.as_secs(),
                stderr.trim()
            )))
        }
    }
}

fn is_retryable_wrangler_failure(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("fetch failed")
        || lower.contains("connectivity")
        || lower.contains("network")
        || lower.contains("timed out")
        || lower.contains("timeout")
}

fn sanitize_wrangler_stderr(stderr: &[u8], token: &str, account_id: &str) -> String {
    let text = String::from_utf8_lossy(stderr)
        .replace(token, "<redacted>")
        .replace(account_id, "<redacted>");
    sanitize_external_text(&text)
}

fn log_event(command: &str, stage: &str, pairs: &[(&str, String)]) {
    home_log_event(
        LogLevel::Info,
        &format!("{command}.{stage}"),
        "wrangler command",
        pairs,
    );
}

fn sanitized_wrangler_operation(args: &[&str]) -> String {
    match args {
        ["r2", "bucket", "list"] => "r2 bucket list".to_string(),
        ["r2", "bucket", "create", bucket] => format!("r2 bucket create {bucket}"),
        ["r2", "bucket", "domain", "get", bucket, "--domain", domain] => {
            format!("r2 bucket domain get {bucket} --domain {domain}")
        }
        [
            "r2",
            "bucket",
            "domain",
            "add",
            bucket,
            "--domain",
            domain,
            "--zone-id",
            _,
            "--min-tls",
            tls,
            "--force",
        ] => {
            format!(
                "r2 bucket domain add {bucket} --domain {domain} --zone-id <derived> --min-tls {tls} --force"
            )
        }
        [
            "r2",
            "bucket",
            "domain",
            "update",
            bucket,
            "--domain",
            domain,
            "--min-tls",
            tls,
        ] => {
            format!("r2 bucket domain update {bucket} --domain {domain} --min-tls {tls}")
        }
        [
            "r2",
            "object",
            "put",
            target,
            "--file",
            file,
            "--content-type",
            _,
            "--cache-control",
            _,
            "--remote",
            "--force",
        ] => {
            format!(
                "r2 object put {} --file {} --content-type <config-json> --cache-control <no-store> --remote --force",
                sanitized_r2_target(target),
                file
            )
        }
        _ => args.join(" "),
    }
}

fn sanitized_r2_target(target: &str) -> String {
    let mut parts: Vec<&str> = target.split('/').collect();
    if parts.len() >= 4 && parts[1] == "config" {
        parts[2] = "<config_key>";
        parts.join("/")
    } else {
        target.to_string()
    }
}

fn bucket_list_contains(output: &str, bucket: &str) -> bool {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output)
        && json_contains_string_or_name(&value, bucket)
    {
        return true;
    }
    output.lines().any(|line| {
        line.split(|c: char| c.is_whitespace() || matches!(c, ',' | '"' | '\'' | '[' | ']'))
            .any(|part| part == bucket)
    })
}

fn json_contains_string_or_name(value: &serde_json::Value, bucket: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text == bucket,
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_contains_string_or_name(child, bucket)),
        serde_json::Value::Object(map) => {
            map.get("name").and_then(serde_json::Value::as_str) == Some(bucket)
                || map
                    .values()
                    .any(|child| json_contains_string_or_name(child, bucket))
        }
        _ => false,
    }
}

fn parse_min_tls(output: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output)
        && let Some(found) = find_min_tls(&value)
    {
        return Some(found);
    }
    if output.contains(R2_CUSTOM_DOMAIN_MIN_TLS) {
        Some(R2_CUSTOM_DOMAIN_MIN_TLS.to_string())
    } else {
        None
    }
}

fn find_min_tls(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for key in [
                "min_tls",
                "minTLS",
                "minimum_tls_version",
                "minimumTlsVersion",
            ] {
                if let Some(text) = map.get(key).and_then(serde_json::Value::as_str) {
                    return Some(text.to_string());
                }
            }
            map.values().find_map(find_min_tls)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_min_tls),
        _ => None,
    }
}

fn looks_like_missing_domain(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("not found")
        || lower.contains("not exist")
        || lower.contains("does not exist")
        || lower.contains("not connected")
        || lower.contains("no such")
}

pub fn public_config_url(delivery_domain: &str, config_key: &str, platform: &str) -> String {
    format!(
        "https://{delivery_domain}/{}",
        public_config_object_key(config_key, platform)
    )
}

pub fn public_config_object_key(config_key: &str, platform: &str) -> String {
    let file = config_variant(platform)
        .map(|entry| entry.public_config_file)
        .unwrap_or("unknown");
    format!("config/{config_key}/{file}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_list_uses_exact_names() {
        assert!(bucket_list_contains(
            r#"[{"name":"yaoe-config"},{"name":"other"}]"#,
            "yaoe-config"
        ));
        assert!(!bucket_list_contains(
            r#"[{"name":"not-yaoe-config"}]"#,
            "yaoe-config"
        ));
    }

    #[test]
    fn parses_min_tls_from_json_or_text() {
        assert_eq!(
            parse_min_tls(r#"{"domain":{"minimum_tls_version":"1.3"}}"#),
            Some("1.3".to_string())
        );
        assert_eq!(
            parse_min_tls("minimum TLS version: 1.2"),
            Some("1.2".to_string())
        );
    }

    #[test]
    fn wrangler_operation_redacts_config_key_from_object_put() {
        let op = sanitized_wrangler_operation(&[
            "r2",
            "object",
            "put",
            "yaoe-config/config/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/ios.json",
            "--file",
            ".yaoe/work/delivery/rendered-config/ios.json",
            "--content-type",
            yaoe_home::R2_JSON_CONTENT_TYPE,
            "--cache-control",
            R2_CONFIG_CACHE_CONTROL,
            "--remote",
            "--force",
        ]);
        assert!(op.contains("yaoe-config/config/<config_key>/ios.json"));
        assert!(!op.contains("AAAAAAAAAAAAAAAA"));
    }

    #[test]
    fn wrangler_stderr_sanitization_removes_ansi_icons_and_secrets() {
        let cleaned = sanitize_wrangler_stderr(
            b"\x1b[33m\xe2\x96\xb2 \x1b[1m[WARNING]\x1b[0m token-123 account-123\n\xf0\x9f\xaa\xb5 fetch failed",
            "token-123",
            "account-123",
        );
        assert_eq!(cleaned, "[WARNING] <redacted> <redacted>\nfetch failed");
        assert!(!cleaned.contains('\u{1b}'));
        assert!(!cleaned.contains('▲'));
        assert!(!cleaned.contains('🪵'));
        assert!(!cleaned.contains("token-123"));
        assert!(!cleaned.contains("account-123"));
    }

    #[test]
    fn wrangler_retry_classification_is_network_only() {
        assert!(is_retryable_wrangler_failure(
            "[ERROR] fetch failed due to connectivity issue"
        ));
        assert!(is_retryable_wrangler_failure("request timed out"));
        assert!(!is_retryable_wrangler_failure(
            "authentication error: token is invalid"
        ));
        assert!(!is_retryable_wrangler_failure(
            "bucket name is not available"
        ));
    }

    #[test]
    fn config_object_key_and_url_match_contract() {
        let key = "A".repeat(128);
        assert_eq!(
            public_config_object_key(&key, "clash-verge"),
            format!("config/{key}/clash-verge.yaml")
        );
        assert_eq!(
            public_config_object_key(&key, "linux-amd64"),
            format!("config/{key}/linux-amd64.json")
        );
        assert_eq!(
            public_config_url("cfg.test.net", &key, "ios"),
            format!("https://cfg.test.net/config/{key}/ios.json")
        );
    }
}
