use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ipnet::IpNet;
use rand::{Rng, RngCore};
use regex::Regex;
use serde::Deserialize;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};
use yaoe_home::{
    BUILTIN_DIRECT_IPV4_CIDRS, CONFIG_KEY_LENGTH, CONFIG_KEY_RANDOM_BYTES, NETBIRD_DIRECT_CIDR,
    REALITY_PRIVATE_KEY_LENGTH, REALITY_PUBLIC_KEY_LENGTH, REALITY_SHORT_ID_BYTES,
    REALITY_SHORT_ID_HEX_LENGTH, SERVER_PORT_MAX, SERVER_PORT_MIN, YaoeError, YaoeResult,
    atomic_write,
};

use crate::model::{Config, ServerConfig};

const SERVER_NAME_RE: &str = r"^[a-z][a-z0-9-]{0,62}$";
const CONFIG_KEY_RE: &str = r"^[A-Za-z0-9_-]{128}$";
const REALITY_KEY_RE: &str = r"^[A-Za-z0-9_-]{43}$";
const REALITY_SHORT_ID_RE: &str = r"^[0-9a-f]{16}$";
const BUCKET_RE: &str = r"^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$";
const IDENT_RE: &str = r"^[A-Za-z0-9_.-]+$";
const ALLOWED_ROOT_KEYS: &[&str] = &[
    "ssh",
    "cloudflare",
    "gitee",
    "credential",
    "reality",
    "route",
    "server",
];

#[derive(Debug, Deserialize)]
struct RawToml {
    #[serde(flatten)]
    tables: BTreeMap<String, toml::Value>,
}

pub fn load_and_validate(path: &Path) -> YaoeResult<Config> {
    let text = fs::read_to_string(path)
        .map_err(|e| YaoeError::State(format!("failed to read {}: {e}", path.display())))?;
    parse_and_validate(&text)
}

pub fn parse_for_client_entrypoints(text: &str) -> YaoeResult<ClientEntrypointParts> {
    reject_unknown_root_keys(text)?;
    reject_forbidden_surfaces(text)?;
    let partial: PartialConfig =
        toml::from_str(text).map_err(|e| YaoeError::Config(format!("TOML parse error: {e}")))?;

    let cloudflare = partial
        .cloudflare
        .ok_or_else(|| YaoeError::Config("[cloudflare].delivery_domain is required".into()))?;
    let delivery_domain = cloudflare
        .delivery_domain
        .ok_or_else(|| YaoeError::Config("[cloudflare].delivery_domain is required".into()))?;
    validate_fqdn("cloudflare.delivery_domain", &delivery_domain)?;
    if delivery_domain.split('.').count() < 3 {
        return Err(YaoeError::Config(
            "cloudflare.delivery_domain must have at least three labels".into(),
        ));
    }
    if is_placeholder(&delivery_domain) {
        return Err(YaoeError::Config(
            "cloudflare.delivery_domain contains a placeholder value".into(),
        ));
    }

    let credential = partial
        .credential
        .ok_or_else(|| YaoeError::Config("[credential].config_key is required".into()))?;
    let config_key = credential
        .config_key
        .ok_or_else(|| YaoeError::Config("[credential].config_key is required".into()))?;
    validate_regex("credential.config_key", &config_key, CONFIG_KEY_RE)?;
    if is_placeholder(&config_key) {
        return Err(YaoeError::Config(
            "credential.config_key contains a placeholder value".into(),
        ));
    }

    let gitee = partial
        .gitee
        .ok_or_else(|| YaoeError::Config("[gitee].owner and [gitee].repo are required".into()))?;
    let gitee_owner = gitee
        .owner
        .ok_or_else(|| YaoeError::Config("[gitee].owner is required".into()))?;
    let gitee_repo = gitee
        .repo
        .ok_or_else(|| YaoeError::Config("[gitee].repo is required".into()))?;
    validate_identifier("gitee.owner", &gitee_owner)?;
    validate_identifier("gitee.repo", &gitee_repo)?;
    if is_placeholder(&gitee_owner) {
        return Err(YaoeError::Config(
            "gitee.owner contains a placeholder value".into(),
        ));
    }
    if is_placeholder(&gitee_repo) {
        return Err(YaoeError::Config(
            "gitee.repo contains a placeholder value".into(),
        ));
    }

    Ok(ClientEntrypointParts {
        delivery_domain,
        config_key,
        gitee_owner,
        gitee_repo,
    })
}

pub fn parse_and_validate(text: &str) -> YaoeResult<Config> {
    reject_unknown_root_keys(text)?;
    reject_forbidden_surfaces(text)?;
    let config: Config =
        toml::from_str(text).map_err(|e| YaoeError::Config(format!("TOML parse error: {e}")))?;
    validate_config(&config)?;
    Ok(config)
}

fn reject_unknown_root_keys(text: &str) -> YaoeResult<()> {
    let raw: RawToml =
        toml::from_str(text).map_err(|e| YaoeError::Config(format!("TOML parse error: {e}")))?;
    let allowed: HashSet<&str> = ALLOWED_ROOT_KEYS.iter().copied().collect();
    for key in raw.tables.keys() {
        if allowed.contains(key.as_str()) {
            continue;
        }
        return Err(YaoeError::Config(format!("unknown table or field: {key}")));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientEntrypointParts {
    pub delivery_domain: String,
    pub config_key: String,
    pub gitee_owner: String,
    pub gitee_repo: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialConfig {
    ssh: Option<PartialSsh>,
    cloudflare: Option<PartialCloudflare>,
    gitee: Option<PartialGitee>,
    credential: Option<PartialCredential>,
    reality: Option<PartialReality>,
    route: Option<PartialRoute>,
    #[serde(default)]
    server: BTreeMap<String, PartialServer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialSsh {
    key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialCloudflare {
    token: Option<String>,
    account_id: Option<String>,
    delivery_domain: Option<String>,
    r2_bucket: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialGitee {
    token: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialCredential {
    vless_uuid: Option<String>,
    config_key: Option<String>,
    reality_private_key: Option<String>,
    reality_short_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialReality {
    handshake_server: Option<String>,
    handshake_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialRoute {
    direct_cidrs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PartialServer {
    ssh: Option<String>,
    ip: Option<String>,
    port: Option<u16>,
    key: Option<String>,
}

fn reject_forbidden_surfaces(text: &str) -> YaoeResult<()> {
    let value: toml::Value =
        toml::from_str(text).map_err(|e| YaoeError::Config(format!("TOML parse error: {e}")))?;
    let forbidden = [
        ("cloudflare", "email"),
        ("cloudflare", "zone"),
        ("cloudflare", "delivery_project"),
        ("cloudflare", "srs_domain"),
        ("cloudflare", "srs_project"),
        ("gitee", "branch"),
        ("gitee", "release_tag"),
        ("credential", "reality_public_key"),
        ("upstream", "runtime_url"),
    ];
    for (table, field) in forbidden {
        if value
            .get(table)
            .and_then(toml::Value::as_table)
            .is_some_and(|t| t.contains_key(field))
        {
            return Err(YaoeError::Config(format!(
                "{table}.{field} is not valid in YAOE v0.0.1"
            )));
        }
    }
    if value.get("upstream").is_some() {
        return Err(YaoeError::Config(
            "[upstream] is not valid in YAOE v0.0.1".into(),
        ));
    }
    if let Some(server) = value.get("server").and_then(toml::Value::as_table) {
        for (name, value) in server {
            if let Some(table) = value.as_table() {
                for field in ["domain", "arch", "platform", "os"] {
                    if table.contains_key(field) {
                        return Err(YaoeError::Config(format!(
                            "server.{name}.{field} is not valid in YAOE v0.0.1"
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn validate_config(config: &Config) -> YaoeResult<()> {
    reject_placeholders(config)?;
    validate_cloudflare(config)?;
    validate_gitee(config)?;
    validate_credentials(config)?;
    validate_reality(config)?;
    validate_servers(config)?;
    validate_route(config)?;
    Ok(())
}

fn reject_placeholders(config: &Config) -> YaoeResult<()> {
    let values = [
        ("cloudflare.token", config.cloudflare.token.as_str()),
        (
            "cloudflare.account_id",
            config.cloudflare.account_id.as_str(),
        ),
        (
            "cloudflare.delivery_domain",
            config.cloudflare.delivery_domain.as_str(),
        ),
        ("gitee.token", config.gitee.token.as_str()),
        ("gitee.owner", config.gitee.owner.as_str()),
        ("gitee.repo", config.gitee.repo.as_str()),
        (
            "credential.vless_uuid",
            config.credential.vless_uuid.as_str(),
        ),
        (
            "credential.config_key",
            config.credential.config_key.as_str(),
        ),
        (
            "credential.reality_private_key",
            config.credential.reality_private_key.as_str(),
        ),
        (
            "reality.handshake_server",
            config.reality.handshake_server.as_str(),
        ),
    ];
    for (field, value) in values {
        if is_placeholder(value) {
            return Err(YaoeError::Config(format!(
                "{field} contains a placeholder value"
            )));
        }
    }
    for (name, server) in &config.server {
        if is_placeholder(&server.ssh) || is_placeholder(&server.ip) {
            return Err(YaoeError::Config(format!(
                "server.{name} contains a placeholder value"
            )));
        }
    }
    Ok(())
}

fn is_placeholder(value: &str) -> bool {
    value.contains("replace_with")
        || value.contains("your-org")
        || value.contains("example.com")
        || value.contains("203.0.113.")
        || value.contains("cf_real_")
        || value.contains("gitee_real_")
        || value == "00000000-0000-4000-8000-000000000000"
}

fn validate_cloudflare(config: &Config) -> YaoeResult<()> {
    let cf = &config.cloudflare;
    non_empty("cloudflare.token", &cf.token)?;
    non_empty("cloudflare.account_id", &cf.account_id)?;
    validate_fqdn("cloudflare.delivery_domain", &cf.delivery_domain)?;
    if cf.delivery_domain.split('.').count() < 3 {
        return Err(YaoeError::Config(
            "cloudflare.delivery_domain must have at least three labels".into(),
        ));
    }
    let bucket_re = Regex::new(BUCKET_RE).expect("valid regex");
    if !bucket_re.is_match(&cf.r2_bucket) {
        return Err(YaoeError::Config(
            "cloudflare.r2_bucket is not a valid R2 bucket name".into(),
        ));
    }
    Ok(())
}

fn validate_gitee(config: &Config) -> YaoeResult<()> {
    non_empty("gitee.token", &config.gitee.token)?;
    validate_identifier("gitee.owner", &config.gitee.owner)?;
    validate_identifier("gitee.repo", &config.gitee.repo)
}

fn validate_credentials(config: &Config) -> YaoeResult<()> {
    Uuid::parse_str(&config.credential.vless_uuid)
        .map_err(|e| YaoeError::Config(format!("credential.vless_uuid is invalid: {e}")))?;
    validate_regex(
        "credential.config_key",
        &config.credential.config_key,
        CONFIG_KEY_RE,
    )?;
    validate_regex(
        "credential.reality_private_key",
        &config.credential.reality_private_key,
        REALITY_KEY_RE,
    )?;
    validate_regex(
        "credential.reality_short_id",
        &config.credential.reality_short_id,
        REALITY_SHORT_ID_RE,
    )?;
    derive_reality_public_key(&config.credential.reality_private_key)?;
    Ok(())
}

fn validate_reality(config: &Config) -> YaoeResult<()> {
    validate_fqdn("reality.handshake_server", &config.reality.handshake_server)?;
    if config.reality.handshake_server.parse::<IpAddr>().is_ok() {
        return Err(YaoeError::Config(
            "reality.handshake_server must not be an IP literal".into(),
        ));
    }
    if config.reality.handshake_port == 0 {
        return Err(YaoeError::Config(
            "reality.handshake_port must be in 1..=65535".into(),
        ));
    }
    Ok(())
}

fn validate_servers(config: &Config) -> YaoeResult<()> {
    if config.server.is_empty() {
        return Err(YaoeError::Config(
            "at least one [server.<name>] is required".into(),
        ));
    }
    let name_re = Regex::new(SERVER_NAME_RE).expect("valid regex");
    let mut ssh_targets = HashSet::new();
    let mut ips = HashSet::new();
    let default_key = config.ssh.as_ref().map(|s| s.key.as_str());
    if let Some(ssh) = &config.ssh {
        validate_path("ssh.key", &ssh.key)?;
    }
    for (name, server) in &config.server {
        if !name_re.is_match(name) {
            return Err(YaoeError::Config(format!(
                "server name {name:?} does not match grammar"
            )));
        }
        validate_server(name, server, default_key)?;
        if !ssh_targets.insert(server.ssh.clone()) {
            return Err(YaoeError::Config(format!(
                "duplicate SSH target for server {name}: {}",
                server.ssh
            )));
        }
        let ip = parse_ipv4("server.ip", &server.ip)?;
        if !ips.insert(ip) {
            return Err(YaoeError::Config(format!(
                "duplicate endpoint IP for server {name}: {}",
                server.ip
            )));
        }
    }
    Ok(())
}

fn validate_server(name: &str, server: &ServerConfig, default_key: Option<&str>) -> YaoeResult<()> {
    validate_ssh_destination(name, &server.ssh)?;
    parse_ipv4(&format!("server.{name}.ip"), &server.ip)?;
    if !(SERVER_PORT_MIN..=SERVER_PORT_MAX).contains(&server.port) {
        return Err(YaoeError::Config(format!(
            "server.{name}.port must be in {SERVER_PORT_MIN}..={SERVER_PORT_MAX}"
        )));
    }
    match server.key.as_deref().or(default_key) {
        Some(key) => validate_path(&format!("server.{name}.key"), key),
        None => Err(YaoeError::Config(
            "[ssh].key is required unless every server defines key".into(),
        )),
    }
}

fn validate_route(config: &Config) -> YaoeResult<()> {
    let mut seen = HashSet::new();
    let mut reserved = HashSet::new();
    for cidr in BUILTIN_DIRECT_IPV4_CIDRS {
        reserved.insert(canonical_cidr(cidr)?);
    }
    reserved.insert(canonical_cidr(NETBIRD_DIRECT_CIDR)?);
    for server in config.server.values() {
        let ip = parse_ipv4("server.ip", &server.ip)?;
        reserved.insert(canonical_cidr(&format!("{ip}/32"))?);
    }
    for cidr in &config.route.direct_cidrs {
        let parsed = cidr
            .parse::<IpNet>()
            .map_err(|e| YaoeError::Config(format!("invalid route.direct_cidrs value: {e}")))?;
        if !matches!(parsed, IpNet::V4(_)) {
            return Err(YaoeError::Config(format!(
                "route.direct_cidrs value must be IPv4: {cidr}"
            )));
        }
        let canonical = parsed.trunc().to_string();
        if reserved.contains(&canonical) {
            return Err(YaoeError::Config(format!(
                "route.direct_cidrs entry duplicates a generated direct route: {canonical}"
            )));
        }
        if !seen.insert(canonical.clone()) {
            return Err(YaoeError::Config(format!(
                "duplicate route.direct_cidrs entry after canonicalization: {canonical}"
            )));
        }
    }
    Ok(())
}

fn canonical_cidr(cidr: &str) -> YaoeResult<String> {
    let parsed: IpNet = cidr
        .parse()
        .map_err(|e| YaoeError::Internal(format!("built-in CIDR parse failed: {e}")))?;
    Ok(parsed.trunc().to_string())
}

fn parse_ipv4(field: &str, ip: &str) -> YaoeResult<Ipv4Addr> {
    ip.parse::<Ipv4Addr>()
        .map_err(|e| YaoeError::Config(format!("{field} is not a valid IPv4 literal: {e}")))
}

fn validate_fqdn(field: &str, domain: &str) -> YaoeResult<()> {
    if domain.is_empty() || domain.ends_with('.') || domain.contains('*') {
        return Err(YaoeError::Config(format!("{field} is not a valid FQDN")));
    }
    if domain.parse::<IpAddr>().is_ok() {
        return Err(YaoeError::Config(format!(
            "{field} must not be an IP literal"
        )));
    }
    if domain != domain.to_ascii_lowercase() {
        return Err(YaoeError::Config(format!(
            "{field} must be lowercase ASCII"
        )));
    }
    if !domain
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
    {
        return Err(YaoeError::Config(format!(
            "{field} contains non-ASCII or invalid DNS characters"
        )));
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return Err(YaoeError::Config(format!("{field} is not a valid FQDN")));
    }
    for label in labels {
        if label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-') {
            return Err(YaoeError::Config(format!(
                "{field} contains an invalid DNS label"
            )));
        }
    }
    Ok(())
}

fn validate_identifier(field: &str, value: &str) -> YaoeResult<()> {
    non_empty(field, value)?;
    if value.starts_with('/') || value.ends_with('/') || value.contains('/') {
        return Err(YaoeError::Config(format!("{field} must not contain slash")));
    }
    if value.contains(char::is_whitespace) || contains_shell_metachar(value) {
        return Err(YaoeError::Config(format!(
            "{field} contains forbidden characters"
        )));
    }
    let re = Regex::new(IDENT_RE).expect("valid regex");
    if !re.is_match(value) {
        return Err(YaoeError::Config(format!(
            "{field} is not a valid ASCII identifier"
        )));
    }
    Ok(())
}

fn validate_path(field: &str, path: &str) -> YaoeResult<()> {
    non_empty(field, path)?;
    if path.trim() != path || path.contains(char::is_whitespace) || contains_shell_metachar(path) {
        return Err(YaoeError::Config(format!(
            "{field} contains forbidden path characters"
        )));
    }
    if path.starts_with("~/") || path.starts_with('/') {
        return Ok(());
    }
    Err(YaoeError::Config(format!(
        "{field} must be absolute or start with ~/"
    )))
}

fn validate_ssh_destination(server_name: &str, ssh: &str) -> YaoeResult<()> {
    non_empty(&format!("server.{server_name}.ssh"), ssh)?;
    let Some(host) = ssh.strip_prefix("root@") else {
        return Err(YaoeError::Config(format!(
            "server.{server_name}.ssh must begin with root@"
        )));
    };
    if host.is_empty()
        || host.contains(':')
        || host.contains(char::is_whitespace)
        || contains_shell_metachar(host)
    {
        return Err(YaoeError::Config(format!(
            "server.{server_name}.ssh is not a plain OpenSSH destination"
        )));
    }
    Ok(())
}

fn validate_regex(field: &str, value: &str, regex: &str) -> YaoeResult<()> {
    let re = Regex::new(regex).expect("valid regex");
    if !re.is_match(value) {
        return Err(YaoeError::Config(format!("{field} has invalid shape")));
    }
    Ok(())
}

fn non_empty(field: &str, value: &str) -> YaoeResult<()> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(YaoeError::Config(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(())
}

fn contains_shell_metachar(value: &str) -> bool {
    value.chars().any(|c| {
        matches!(
            c,
            '$' | '`'
                | ';'
                | '|'
                | '&'
                | '>'
                | '<'
                | '*'
                | '?'
                | '\''
                | '"'
                | '('
                | ')'
                | '{'
                | '}'
                | '['
                | ']'
                | '\\'
                | '\n'
                | '\r'
                | '\t'
        )
    })
}

pub fn derive_reality_public_key(private_key: &str) -> YaoeResult<String> {
    if private_key.len() != REALITY_PRIVATE_KEY_LENGTH {
        return Err(YaoeError::Config(
            "credential.reality_private_key has invalid length".into(),
        ));
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(private_key)
        .map_err(|e| YaoeError::Config(format!("decode Reality private key: {e}")))?;
    let bytes: [u8; 32] = decoded.try_into().map_err(|_| {
        YaoeError::Config("credential.reality_private_key must decode to 32 bytes".into())
    })?;
    let secret = StaticSecret::from(bytes);
    let public = PublicKey::from(&secret);
    let encoded = URL_SAFE_NO_PAD.encode(public.as_bytes());
    if encoded.len() != REALITY_PUBLIC_KEY_LENGTH {
        return Err(YaoeError::Config(
            "derived Reality public key has invalid length".into(),
        ));
    }
    Ok(encoded)
}

pub fn generate_config_key() -> String {
    let mut bytes = [0u8; CONFIG_KEY_RANDOM_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    let key = URL_SAFE_NO_PAD.encode(bytes);
    debug_assert_eq!(key.len(), CONFIG_KEY_LENGTH);
    key
}

pub fn generate_vless_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn generate_reality_short_id() -> String {
    let mut bytes = [0u8; REALITY_SHORT_ID_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    let id = hex::encode(bytes);
    debug_assert_eq!(id.len(), REALITY_SHORT_ID_HEX_LENGTH);
    id
}

pub fn generate_server_port() -> u16 {
    rand::thread_rng().gen_range(SERVER_PORT_MIN..=SERVER_PORT_MAX)
}

pub fn atomic_update_toml_field(
    path: &Path,
    table: &str,
    field: &str,
    value: &str,
) -> YaoeResult<()> {
    let text = fs::read_to_string(path)
        .map_err(|e| YaoeError::State(format!("read {}: {e}", path.display())))?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| YaoeError::Config(format!("TOML parse error: {e}")))?;
    update_existing_string_field(&mut doc, table, field, value)?;
    atomic_write(path, doc.to_string().as_bytes(), 0o600)
}

pub fn atomic_update_reality_keypair(
    path: &Path,
    private_key: &str,
    short_id: &str,
) -> YaoeResult<()> {
    let text = fs::read_to_string(path)
        .map_err(|e| YaoeError::State(format!("read {}: {e}", path.display())))?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| YaoeError::Config(format!("TOML parse error: {e}")))?;
    update_existing_string_field(&mut doc, "credential", "reality_private_key", private_key)?;
    update_existing_string_field(&mut doc, "credential", "reality_short_id", short_id)?;
    atomic_write(path, doc.to_string().as_bytes(), 0o600)
}

fn update_existing_string_field(
    doc: &mut toml_edit::DocumentMut,
    table: &str,
    field: &str,
    value: &str,
) -> YaoeResult<()> {
    let table_item = doc
        .get_mut(table)
        .and_then(toml_edit::Item::as_table_mut)
        .ok_or_else(|| {
            YaoeError::Config(format!("[{table}] is required for credential rotation"))
        })?;
    let field_item = table_item.get_mut(field).ok_or_else(|| {
        YaoeError::Config(format!(
            "{table}.{field} is required for credential rotation"
        ))
    })?;
    if field_item
        .as_value()
        .and_then(toml_edit::Value::as_str)
        .is_none()
    {
        return Err(YaoeError::Config(format!(
            "{table}.{field} must be a string value for credential rotation"
        )));
    }
    *field_item = toml_edit::value(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> String {
        let private = "sBFVzOLPGBcFMR55fkt7_5xY5SGVD7Vsw8jrLEObh1I";
        format!(
            r#"[ssh]
key = "~/.ssh/id_ed25519"

[cloudflare]
token = "token"
account_id = "account"
delivery_domain = "cfg.test.net"
r2_bucket = "yaoe-config"

[gitee]
token = "token"
owner = "owner"
repo = "repo"

[credential]
vless_uuid = "550e8400-e29b-41d4-a716-446655440000"
config_key = "{}"
reality_private_key = "{private}"
reality_short_id = "0123456789abcdef"

[reality]
handshake_server = "www.cloudflare.com"

[server.hk]
ssh = "root@198.51.100.10"
ip = "198.51.100.10"
port = 28443
"#,
            generate_config_key()
        )
    }

    #[test]
    fn accepts_valid_config() {
        let config = parse_and_validate(&valid_config()).unwrap();
        assert!(config.route.direct_cidrs.is_empty());
    }

    #[test]
    fn rejects_user_direct_cidr_duplicates_generated_routes() {
        for cidr in ["100.64.0.0/10", "10.0.0.0/8", "198.51.100.10/32"] {
            let text = valid_config().replace(
                "[server.hk]",
                &format!("[route]\ndirect_cidrs = [\"{cidr}\"]\n\n[server.hk]"),
            );
            let err = parse_and_validate(&text).unwrap_err().to_string();
            assert!(err.contains("route.direct_cidrs"));
            assert!(err.contains("duplicates"));
        }
    }

    #[test]
    fn rejects_removed_server_domain() {
        let text = valid_config().replace("port = 28443", "port = 28443\ndomain = \"hk.test.net\"");
        let err = parse_and_validate(&text).unwrap_err().to_string();
        assert!(err.contains("server.hk.domain"));
    }

    #[test]
    fn rejects_user_configured_platform_surfaces() {
        for (needle, text) in [
            ("platform", valid_config() + "\n[platform]\nlinux = true\n"),
            (
                "server.hk.os",
                valid_config().replace("port = 28443", "port = 28443\nos = \"linux\""),
            ),
            (
                "server.hk.arch",
                valid_config().replace("port = 28443", "port = 28443\narch = \"amd64\""),
            ),
            (
                "credential.reality_public_key",
                valid_config().replace(
                    "reality_short_id = \"0123456789abcdef\"",
                    "reality_short_id = \"0123456789abcdef\"\nreality_public_key = \"abc\"",
                ),
            ),
        ] {
            let err = parse_and_validate(&text).unwrap_err().to_string();
            assert!(
                err.contains(needle),
                "error {err:?} did not mention {needle}"
            );
        }
    }

    #[test]
    fn rejects_ip_literal_delivery_domain() {
        let text = valid_config().replace(
            "delivery_domain = \"cfg.test.net\"",
            "delivery_domain = \"1.2.3.4\"",
        );
        let err = parse_and_validate(&text).unwrap_err().to_string();
        assert!(err.contains("cloudflare.delivery_domain"));
    }

    #[test]
    fn rejects_ipv6_server_endpoint() {
        let text = valid_config().replace("ip = \"198.51.100.10\"", "ip = \"2001:db8::10\"");
        let err = parse_and_validate(&text).unwrap_err().to_string();
        assert!(err.contains("server.hk.ip"));
        assert!(err.contains("IPv4"));
    }

    #[test]
    fn rejects_ipv6_direct_cidr() {
        let text = valid_config().replace(
            "[server.hk]",
            "[route]\ndirect_cidrs = [\"fd7a:115c:a1e0::/48\"]\n\n[server.hk]",
        );
        let err = parse_and_validate(&text).unwrap_err().to_string();
        assert!(err.contains("route.direct_cidrs"));
        assert!(err.contains("IPv4"));
    }

    #[test]
    fn rejects_ssh_destination_with_inline_port() {
        let text = valid_config().replace(
            "ssh = \"root@198.51.100.10\"",
            "ssh = \"root@198.51.100.10:2222\"",
        );
        let err = parse_and_validate(&text).unwrap_err().to_string();
        assert!(err.contains("server.hk.ssh"));
    }

    #[test]
    fn generated_config_key_shape_is_fixed() {
        let key = generate_config_key();
        assert_eq!(key.len(), CONFIG_KEY_LENGTH);
        assert!(Regex::new(CONFIG_KEY_RE).unwrap().is_match(&key));
    }

    #[test]
    fn client_entrypoint_profile_requires_gitee_coordinates() {
        let key = "A".repeat(CONFIG_KEY_LENGTH);
        let text = format!(
            r#"[cloudflare]
delivery_domain = "cfg.test.net"

[credential]
config_key = "{key}"
"#
        );
        let err = parse_for_client_entrypoints(&text).unwrap_err().to_string();
        assert!(err.contains("gitee"));

        let text = format!(
            r#"[cloudflare]
delivery_domain = "cfg.test.net"

[gitee]
owner = "owner"
repo = "repo"

[credential]
config_key = "{key}"
"#
        );
        let parts = parse_for_client_entrypoints(&text).unwrap();
        assert_eq!(parts.gitee_owner, "owner");
        assert_eq!(parts.gitee_repo, "repo");
    }

    #[test]
    fn rotation_updates_preserve_unrelated_toml_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("yaoe.toml");
        let text = format!(
            r#"# operator comment
[credential]
vless_uuid = "550e8400-e29b-41d4-a716-446655440000" # keep uuid comment
config_key = "{}"
reality_private_key = "sBFVzOLPGBcFMR55fkt7_5xY5SGVD7Vsw8jrLEObh1I"
reality_short_id = "0123456789abcdef"

[gitee]
owner = "owner"
"#,
            "A".repeat(CONFIG_KEY_LENGTH)
        );
        fs::write(&path, text).unwrap();

        let new_key = "B".repeat(CONFIG_KEY_LENGTH);
        atomic_update_toml_field(&path, "credential", "config_key", &new_key).unwrap();
        let updated = fs::read_to_string(&path).unwrap();

        assert!(updated.contains("# operator comment"));
        assert!(updated.contains("# keep uuid comment"));
        assert!(updated.contains("owner = \"owner\""));
        assert!(updated.contains(&format!("config_key = \"{new_key}\"")));
        assert!(updated.contains("vless_uuid = \"550e8400-e29b-41d4-a716-446655440000\""));
    }

    #[test]
    fn reality_rotation_updates_only_private_key_and_short_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("yaoe.toml");
        let old_key = "A".repeat(CONFIG_KEY_LENGTH);
        fs::write(
            &path,
            format!(
                r#"[credential]
vless_uuid = "550e8400-e29b-41d4-a716-446655440000"
config_key = "{old_key}"
reality_private_key = "sBFVzOLPGBcFMR55fkt7_5xY5SGVD7Vsw8jrLEObh1I"
reality_short_id = "0123456789abcdef"
"#
            ),
        )
        .unwrap();

        atomic_update_reality_keypair(
            &path,
            "gIV90X9k0-LaSgI4aomnmkAFGb5U_i1_5kLWTSX0nHk",
            "fedcba9876543210",
        )
        .unwrap();
        let updated = fs::read_to_string(&path).unwrap();

        assert!(updated.contains("vless_uuid = \"550e8400-e29b-41d4-a716-446655440000\""));
        assert!(updated.contains(&format!("config_key = \"{old_key}\"")));
        assert!(
            updated
                .contains("reality_private_key = \"gIV90X9k0-LaSgI4aomnmkAFGb5U_i1_5kLWTSX0nHk\"")
        );
        assert!(updated.contains("reality_short_id = \"fedcba9876543210\""));
        assert!(!updated.contains("0123456789abcdef"));
    }

    #[test]
    fn rotation_rejects_missing_credential_fields_instead_of_creating_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("yaoe.toml");
        fs::write(&path, "[gitee]\nowner = \"owner\"\n").unwrap();

        let err = atomic_update_toml_field(&path, "credential", "config_key", &"B".repeat(128))
            .unwrap_err()
            .to_string();

        assert!(err.contains("[credential] is required"));
        let unchanged = fs::read_to_string(&path).unwrap();
        assert_eq!(unchanged, "[gitee]\nowner = \"owner\"\n");
    }
}
