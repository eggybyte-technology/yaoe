use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub ssh: Option<SshConfig>,
    pub cloudflare: CloudflareConfig,
    pub gitee: GiteeConfig,
    pub credential: CredentialConfig,
    pub reality: RealityConfig,
    #[serde(default)]
    pub route: RouteConfig,
    pub server: BTreeMap<String, ServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshConfig {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudflareConfig {
    pub token: String,
    pub account_id: String,
    pub delivery_domain: String,
    pub r2_bucket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GiteeConfig {
    pub token: String,
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialConfig {
    pub vless_uuid: String,
    pub config_key: String,
    pub reality_private_key: String,
    pub reality_short_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealityConfig {
    pub handshake_server: String,
    #[serde(default = "default_handshake_port")]
    pub handshake_port: u16,
}

fn default_handshake_port() -> u16 {
    443
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteConfig {
    #[serde(default = "default_direct_cidrs")]
    pub direct_cidrs: Vec<String>,
}

pub fn default_direct_cidrs() -> Vec<String> {
    vec!["100.64.0.0/10".to_string()]
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self {
            direct_cidrs: default_direct_cidrs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub ssh: String,
    pub ip: String,
    pub port: u16,
    pub key: Option<String>,
}

impl Config {
    pub fn server_names(&self) -> Vec<String> {
        self.server.keys().cloned().collect()
    }
}
