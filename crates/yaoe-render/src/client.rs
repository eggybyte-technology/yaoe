use std::collections::HashSet;
use std::net::Ipv4Addr;

use ipnet::IpNet;
use serde::Serialize;
use serde_json::Value;
use yaoe_config::{Config, derive_reality_public_key};
use yaoe_home::{
    BUILTIN_DIRECT_IPV4_CIDRS, BUILTIN_DIRECT_IPV6_CIDRS, CN_DNS_PATH, CN_DNS_PORT, CN_DNS_SERVER,
    CN_DNS_TLS_SERVER_NAME, CN_DNS_TYPE, CN_DOMAIN_PUBLIC_ASSET, CN_DOMAIN_RULE_TAG,
    CN_IPV4_PUBLIC_ASSET, CN_IPV4_RULE_TAG, DNS_HIJACK_PORT, DNS_STRATEGY, GITEE_RELEASE_TAG,
    MIHOMO_ALLOW_LAN, MIHOMO_DEFAULT_NAMESERVER, MIHOMO_DIRECT_NAMESERVER, MIHOMO_DNS_ENABLE,
    MIHOMO_DNS_ENHANCED_MODE, MIHOMO_DNS_IPV6, MIHOMO_FAKE_IP_RANGE, MIHOMO_FALLBACK,
    MIHOMO_FALLBACK_FILTER_DOMAIN, MIHOMO_FALLBACK_FILTER_GEOIP, MIHOMO_FALLBACK_FILTER_GEOIP_CODE,
    MIHOMO_FALLBACK_FILTER_GEOSITE, MIHOMO_GEO_AUTO_UPDATE, MIHOMO_GEO_UPDATE_INTERVAL_HOURS,
    MIHOMO_GEODATA_MODE, MIHOMO_GEOIP_URL, MIHOMO_GEOSITE_URL, MIHOMO_IPV6, MIHOMO_LOG_LEVEL,
    MIHOMO_MIXED_PORT, MIHOMO_MMDB_URL, MIHOMO_MODE, MIHOMO_NAMESERVER,
    MIHOMO_TUN_AUTO_DETECT_INTERFACE, MIHOMO_TUN_AUTO_ROUTE, MIHOMO_TUN_DNS_HIJACK,
    MIHOMO_TUN_ENABLE, MIHOMO_TUN_STACK, MIHOMO_TUN_STRICT_ROUTE, MIHOMO_URL_TEST_INTERVAL_SECONDS,
    MIHOMO_URL_TEST_URL, NETBIRD_DIRECT_CIDR, NETBIRD_DOMAIN_EXACT, NETBIRD_DOMAIN_SUFFIX,
    NETBIRD_MIHOMO_FAKE_IP_FILTER, NETBIRD_PROCESS_NAMES, PUBLIC_IPV6_DENIAL_NO_DROP,
    REMOTE_DNS_PATH, REMOTE_DNS_PORT, REMOTE_DNS_SERVER, REMOTE_DNS_TLS_SERVER_NAME,
    REMOTE_DNS_TYPE, TUN_IPV4_ADDRESS, TUN_IPV6_ADDRESS, YaoeError, YaoeResult, config_variant,
};

#[derive(Debug, Clone)]
pub struct ClientRenderInput {
    pub config: Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientPlatform {
    LinuxAmd64,
    LinuxArm64,
    MacosAmd64,
    MacosArm64,
    Ios,
    Android,
}

impl ClientPlatform {
    pub fn from_config_platform(platform: &str) -> Option<Self> {
        match platform {
            "linux-amd64" => Some(Self::LinuxAmd64),
            "linux-arm64" => Some(Self::LinuxArm64),
            "macos-amd64" => Some(Self::MacosAmd64),
            "macos-arm64" => Some(Self::MacosArm64),
            "ios" => Some(Self::Ios),
            "android" => Some(Self::Android),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinuxAmd64 => "linux-amd64",
            Self::LinuxArm64 => "linux-arm64",
            Self::MacosAmd64 => "macos-amd64",
            Self::MacosArm64 => "macos-arm64",
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }

    pub fn is_linux(self) -> bool {
        matches!(self, Self::LinuxAmd64 | Self::LinuxArm64)
    }

    pub fn is_mobile(self) -> bool {
        matches!(self, Self::Ios | Self::Android)
    }

    pub fn is_service(self) -> bool {
        !self.is_mobile()
    }
}

pub fn render_linux_client_config(input: &ClientRenderInput) -> YaoeResult<String> {
    render_client_config(input, ClientPlatform::LinuxAmd64)
}

pub fn render_macos_client_config(input: &ClientRenderInput) -> YaoeResult<String> {
    render_client_config(input, ClientPlatform::MacosArm64)
}

pub fn render_ios_client_config(input: &ClientRenderInput) -> YaoeResult<String> {
    render_client_config(input, ClientPlatform::Ios)
}

pub fn render_android_client_config(input: &ClientRenderInput) -> YaoeResult<String> {
    render_client_config(input, ClientPlatform::Android)
}

pub fn render_clash_verge_profile(input: &ClientRenderInput) -> YaoeResult<String> {
    let server_names: Vec<String> = input.config.server.keys().cloned().collect();
    if server_names.is_empty() {
        return Err(YaoeError::Internal(
            "clash-verge render requires at least one server".into(),
        ));
    }
    let public_key = derive_reality_public_key(&input.config.credential.reality_private_key)?;
    let mut out = String::new();
    out.push_str(&format!("mixed-port: {MIHOMO_MIXED_PORT}\n"));
    out.push_str(&format!("allow-lan: {}\n", yaml_bool(MIHOMO_ALLOW_LAN)));
    out.push_str(&format!("mode: {MIHOMO_MODE}\n"));
    out.push_str(&format!("log-level: {MIHOMO_LOG_LEVEL}\n"));
    out.push_str(&format!("ipv6: {}\n", yaml_bool(MIHOMO_IPV6)));
    out.push_str(&format!(
        "geodata-mode: {}\n",
        yaml_bool(MIHOMO_GEODATA_MODE)
    ));
    out.push_str(&format!(
        "geo-auto-update: {}\n",
        yaml_bool(MIHOMO_GEO_AUTO_UPDATE)
    ));
    out.push_str(&format!(
        "geo-update-interval: {MIHOMO_GEO_UPDATE_INTERVAL_HOURS}\n"
    ));
    out.push_str("geox-url:\n");
    out.push_str(&format!("  geoip: {MIHOMO_GEOIP_URL}\n"));
    out.push_str(&format!("  geosite: {MIHOMO_GEOSITE_URL}\n"));
    out.push_str(&format!("  mmdb: {MIHOMO_MMDB_URL}\n"));
    out.push_str("dns:\n");
    out.push_str(&format!("  enable: {}\n", yaml_bool(MIHOMO_DNS_ENABLE)));
    out.push_str(&format!("  ipv6: {}\n", yaml_bool(MIHOMO_DNS_IPV6)));
    out.push_str(&format!("  enhanced-mode: {MIHOMO_DNS_ENHANCED_MODE}\n"));
    out.push_str(&format!("  fake-ip-range: {MIHOMO_FAKE_IP_RANGE}\n"));
    out.push_str("  default-nameserver:\n");
    for server in MIHOMO_DEFAULT_NAMESERVER {
        out.push_str(&format!("    - {server}\n"));
    }
    out.push_str("  fake-ip-filter:\n");
    for filter in NETBIRD_MIHOMO_FAKE_IP_FILTER {
        out.push_str(&format!("    - \"{filter}\"\n"));
    }
    out.push_str("  nameserver-policy:\n");
    for policy in ["+.netbird.io", "+.netbird.cloud", "geosite:cn"] {
        out.push_str(&format!("    \"{policy}\":\n"));
        for server in MIHOMO_NAMESERVER {
            out.push_str(&format!("      - {server}\n"));
        }
    }
    out.push_str("  nameserver:\n");
    for server in MIHOMO_NAMESERVER {
        out.push_str(&format!("    - {server}\n"));
    }
    out.push_str("  direct-nameserver:\n");
    for server in MIHOMO_DIRECT_NAMESERVER {
        out.push_str(&format!("    - {server}\n"));
    }
    out.push_str("  direct-nameserver-follow-policy: true\n");
    out.push_str("  fallback:\n");
    for server in MIHOMO_FALLBACK {
        out.push_str(&format!("    - {server}\n"));
    }
    out.push_str("  fallback-filter:\n");
    out.push_str(&format!(
        "    geoip: {}\n",
        yaml_bool(MIHOMO_FALLBACK_FILTER_GEOIP)
    ));
    out.push_str(&format!(
        "    geoip-code: {MIHOMO_FALLBACK_FILTER_GEOIP_CODE}\n"
    ));
    out.push_str("    geosite:\n");
    for geosite in MIHOMO_FALLBACK_FILTER_GEOSITE {
        out.push_str(&format!("      - {geosite}\n"));
    }
    out.push_str("    domain:\n");
    for domain in MIHOMO_FALLBACK_FILTER_DOMAIN {
        out.push_str(&format!("      - \"{domain}\"\n"));
    }
    out.push_str("tun:\n");
    out.push_str(&format!("  enable: {}\n", yaml_bool(MIHOMO_TUN_ENABLE)));
    out.push_str(&format!("  stack: {MIHOMO_TUN_STACK}\n"));
    out.push_str(&format!(
        "  auto-route: {}\n",
        yaml_bool(MIHOMO_TUN_AUTO_ROUTE)
    ));
    out.push_str(&format!(
        "  auto-detect-interface: {}\n",
        yaml_bool(MIHOMO_TUN_AUTO_DETECT_INTERFACE)
    ));
    out.push_str(&format!(
        "  strict-route: {}\n",
        yaml_bool(MIHOMO_TUN_STRICT_ROUTE)
    ));
    out.push_str("  dns-hijack:\n");
    for hijack in MIHOMO_TUN_DNS_HIJACK {
        out.push_str(&format!("    - {hijack}\n"));
    }
    out.push_str("  route-exclude-address:\n");
    for cidr in direct_ipv4_cidrs(&input.config, &server_names)? {
        out.push_str(&format!("    - {cidr}\n"));
    }
    out.push_str("proxies:\n");
    for name in &server_names {
        let server = &input.config.server[name];
        out.push_str(&format!("  - name: egress-{name}\n"));
        out.push_str("    type: vless\n");
        out.push_str(&format!("    server: {}\n", server.ip));
        out.push_str(&format!("    port: {}\n", server.port));
        out.push_str(&format!(
            "    uuid: {}\n",
            input.config.credential.vless_uuid
        ));
        out.push_str("    network: tcp\n");
        out.push_str("    tls: true\n");
        out.push_str("    udp: true\n");
        out.push_str("    flow: xtls-rprx-vision\n");
        out.push_str(&format!(
            "    servername: {}\n",
            input.config.reality.handshake_server
        ));
        out.push_str("    reality-opts:\n");
        out.push_str(&format!("      public-key: {public_key}\n"));
        out.push_str(&format!(
            "      short-id: {}\n",
            input.config.credential.reality_short_id
        ));
        out.push_str("      support-x25519mlkem768: false\n");
        out.push_str("    client-fingerprint: chrome\n");
    }
    out.push_str("proxy-groups:\n");
    out.push_str("  - name: PROXY\n");
    out.push_str("    type: url-test\n");
    out.push_str("    proxies:\n");
    for name in &server_names {
        out.push_str(&format!("      - egress-{name}\n"));
    }
    out.push_str(&format!("    url: {MIHOMO_URL_TEST_URL}\n"));
    out.push_str(&format!(
        "    interval: {MIHOMO_URL_TEST_INTERVAL_SECONDS}\n"
    ));
    out.push_str("rules:\n");
    for process in NETBIRD_PROCESS_NAMES {
        out.push_str(&format!("  - PROCESS-NAME,{process},DIRECT\n"));
    }
    for cidr in direct_ipv4_cidrs(&input.config, &server_names)? {
        out.push_str(&format!("  - IP-CIDR,{cidr},DIRECT,no-resolve\n"));
    }
    for domain in NETBIRD_DOMAIN_EXACT {
        out.push_str(&format!("  - DOMAIN,{domain},DIRECT\n"));
    }
    for suffix in NETBIRD_DOMAIN_SUFFIX {
        out.push_str(&format!("  - DOMAIN-SUFFIX,{suffix},DIRECT\n"));
    }
    out.push_str("  - GEOSITE,cn,DIRECT\n");
    out.push_str("  - GEOIP,CN,DIRECT\n");
    out.push_str("  - MATCH,PROXY\n");
    Ok(out)
}

fn yaml_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub fn render_client_config(
    input: &ClientRenderInput,
    platform: ClientPlatform,
) -> YaoeResult<String> {
    let registry_entry = config_variant(platform.as_str()).ok_or_else(|| {
        YaoeError::Internal(format!("unknown config variant {}", platform.as_str()))
    })?;
    let server_names: Vec<String> = input.config.server.keys().cloned().collect();
    if server_names.is_empty() {
        return Err(YaoeError::Internal(
            "client render requires at least one server".into(),
        ));
    }
    let public_key = derive_reality_public_key(&input.config.credential.reality_private_key)?;
    let outbound_tags: Vec<String> = server_names
        .iter()
        .map(|name| format!("egress-{name}"))
        .collect();
    let mut outbounds = vec![Outbound::UrlTest(UrlTestOutbound {
        kind: "urltest",
        tag: "proxy".to_string(),
        outbounds: outbound_tags,
    })];
    for name in &server_names {
        let server = &input.config.server[name];
        outbounds.push(Outbound::Vless(VlessOutbound {
            kind: "vless",
            tag: format!("egress-{name}"),
            server: server.ip.clone(),
            server_port: server.port,
            uuid: input.config.credential.vless_uuid.clone(),
            flow: "xtls-rprx-vision",
            tls: VlessTls {
                enabled: true,
                server_name: input.config.reality.handshake_server.clone(),
                utls: Utls {
                    enabled: true,
                    fingerprint: "chrome",
                },
                reality: RealityTls {
                    enabled: true,
                    public_key: public_key.clone(),
                    short_id: input.config.credential.reality_short_id.clone(),
                },
            },
        }));
    }
    outbounds.push(Outbound::Direct(DirectOutbound {
        kind: "direct",
        tag: "direct",
    }));

    let doc = ClientConfigDoc {
        log: Log { level: "info" },
        dns: Dns {
            servers: vec![
                DnsServer {
                    kind: CN_DNS_TYPE,
                    tag: "cn-dns",
                    server: CN_DNS_SERVER,
                    server_port: CN_DNS_PORT,
                    path: CN_DNS_PATH,
                    detour: None,
                    tls: DnsTls {
                        server_name: CN_DNS_TLS_SERVER_NAME,
                    },
                },
                DnsServer {
                    kind: REMOTE_DNS_TYPE,
                    tag: "remote-dns",
                    server: REMOTE_DNS_SERVER,
                    server_port: REMOTE_DNS_PORT,
                    path: REMOTE_DNS_PATH,
                    detour: Some("proxy"),
                    tls: DnsTls {
                        server_name: REMOTE_DNS_TLS_SERVER_NAME,
                    },
                },
            ],
            rules: vec![
                DnsRule::Domain(DnsDomainRule {
                    domain: NETBIRD_DOMAIN_EXACT.to_vec(),
                    action: "route",
                    server: "cn-dns",
                }),
                DnsRule::DomainSuffix(DnsDomainSuffixRule {
                    domain_suffix: NETBIRD_DOMAIN_SUFFIX.to_vec(),
                    action: "route",
                    server: "cn-dns",
                }),
                DnsRule::RuleSet(DnsRuleSetRule {
                    rule_set: vec!["cn-domain"],
                    action: "route",
                    server: "cn-dns",
                }),
            ],
            final_out: "remote-dns",
            strategy: DNS_STRATEGY,
            reverse_mapping: true,
        },
        inbounds: vec![TunInbound {
            kind: "tun",
            tag: "tun-in",
            address: vec![TUN_IPV4_ADDRESS, TUN_IPV6_ADDRESS],
            mtu: 1500,
            auto_route: true,
            auto_redirect: (registry_entry.tun_profile == "linux-service").then_some(true),
            strict_route: (registry_entry.tun_profile != "mobile").then_some(true),
            route_exclude_address: direct_cidrs(&input.config, &server_names)?,
        }],
        outbounds,
        route: Route {
            auto_detect_interface: (registry_entry.route_profile == "service").then_some(true),
            default_domain_resolver: "remote-dns",
            rule_set: vec![
                RemoteRuleSet {
                    kind: "remote",
                    tag: CN_DOMAIN_RULE_TAG,
                    format: "binary",
                    url: release_asset_url(
                        &input.config.gitee.owner,
                        &input.config.gitee.repo,
                        CN_DOMAIN_PUBLIC_ASSET,
                    ),
                    download_detour: "direct",
                    update_interval: "1d",
                },
                RemoteRuleSet {
                    kind: "remote",
                    tag: CN_IPV4_RULE_TAG,
                    format: "binary",
                    url: release_asset_url(
                        &input.config.gitee.owner,
                        &input.config.gitee.repo,
                        CN_IPV4_PUBLIC_ASSET,
                    ),
                    download_detour: "direct",
                    update_interval: "1d",
                },
            ],
            rules: route_rules(
                &input.config,
                &server_names,
                registry_entry.route_profile,
                registry_entry.tun_profile,
            )?,
            final_out: "proxy",
        },
    };

    let value = serde_json::to_value(&doc)
        .map_err(|e| YaoeError::Internal(format!("render client value: {e}")))?;
    validate_client_semantics(&value, platform)?;
    let mut rendered = serde_json::to_string_pretty(&doc)
        .map_err(|e| YaoeError::Internal(format!("render client config: {e}")))?;
    rendered.push('\n');
    Ok(rendered)
}

#[derive(Serialize)]
struct ClientConfigDoc {
    log: Log,
    dns: Dns,
    inbounds: Vec<TunInbound>,
    outbounds: Vec<Outbound>,
    route: Route,
}

#[derive(Serialize)]
struct Log {
    level: &'static str,
}

#[derive(Serialize)]
struct Dns {
    servers: Vec<DnsServer>,
    rules: Vec<DnsRule>,
    #[serde(rename = "final")]
    final_out: &'static str,
    strategy: &'static str,
    reverse_mapping: bool,
}

#[derive(Serialize)]
struct DnsServer {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
    server: &'static str,
    server_port: u16,
    path: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detour: Option<&'static str>,
    tls: DnsTls,
}

#[derive(Serialize)]
struct DnsTls {
    server_name: &'static str,
}

#[derive(Serialize)]
#[serde(untagged)]
enum DnsRule {
    Domain(DnsDomainRule),
    DomainSuffix(DnsDomainSuffixRule),
    RuleSet(DnsRuleSetRule),
}

#[derive(Serialize)]
struct DnsDomainRule {
    domain: Vec<&'static str>,
    action: &'static str,
    server: &'static str,
}

#[derive(Serialize)]
struct DnsDomainSuffixRule {
    domain_suffix: Vec<&'static str>,
    action: &'static str,
    server: &'static str,
}

#[derive(Serialize)]
struct DnsRuleSetRule {
    rule_set: Vec<&'static str>,
    action: &'static str,
    server: &'static str,
}

#[derive(Serialize)]
struct TunInbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
    address: Vec<&'static str>,
    mtu: u16,
    auto_route: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_redirect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strict_route: Option<bool>,
    route_exclude_address: Vec<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Outbound {
    UrlTest(UrlTestOutbound),
    Vless(VlessOutbound),
    Direct(DirectOutbound),
}

#[derive(Serialize)]
struct UrlTestOutbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: String,
    outbounds: Vec<String>,
}

#[derive(Serialize)]
struct VlessOutbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: String,
    server: String,
    server_port: u16,
    uuid: String,
    flow: &'static str,
    tls: VlessTls,
}

#[derive(Serialize)]
struct VlessTls {
    enabled: bool,
    server_name: String,
    utls: Utls,
    reality: RealityTls,
}

#[derive(Serialize)]
struct Utls {
    enabled: bool,
    fingerprint: &'static str,
}

#[derive(Serialize)]
struct RealityTls {
    enabled: bool,
    public_key: String,
    short_id: String,
}

#[derive(Serialize)]
struct DirectOutbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
}

#[derive(Serialize)]
struct Route {
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_detect_interface: Option<bool>,
    default_domain_resolver: &'static str,
    rule_set: Vec<RemoteRuleSet>,
    rules: Vec<RouteRule>,
    #[serde(rename = "final")]
    final_out: &'static str,
}

#[derive(Serialize)]
struct RemoteRuleSet {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
    format: &'static str,
    url: String,
    download_detour: &'static str,
    update_interval: &'static str,
}

#[derive(Serialize)]
#[serde(untagged)]
enum RouteRule {
    ProcessName(ProcessNameRouteRule),
    DnsHijack(DnsHijackRouteRule),
    Sniff(SniffRouteRule),
    Domain(DomainRouteRule),
    DomainSuffix(DomainSuffixRouteRule),
    Cidr(CidrRouteRule),
    Ipv6Reject(Ipv6RejectRouteRule),
    RuleSet(RuleSetRouteRule),
}

#[derive(Serialize)]
struct ProcessNameRouteRule {
    process_name: Vec<&'static str>,
    action: &'static str,
    outbound: &'static str,
}

#[derive(Serialize)]
struct DnsHijackRouteRule {
    port: u16,
    action: &'static str,
}

#[derive(Serialize)]
struct SniffRouteRule {
    action: &'static str,
}

#[derive(Serialize)]
struct DomainRouteRule {
    domain: Vec<&'static str>,
    action: &'static str,
    outbound: &'static str,
}

#[derive(Serialize)]
struct DomainSuffixRouteRule {
    domain_suffix: Vec<&'static str>,
    action: &'static str,
    outbound: &'static str,
}

#[derive(Serialize)]
struct CidrRouteRule {
    ip_cidr: Vec<String>,
    action: &'static str,
    outbound: &'static str,
}

#[derive(Serialize)]
struct Ipv6RejectRouteRule {
    ip_version: u8,
    action: &'static str,
    method: &'static str,
    no_drop: bool,
}

#[derive(Serialize)]
struct RuleSetRouteRule {
    rule_set: Vec<&'static str>,
    action: &'static str,
    outbound: &'static str,
}

fn release_asset_url(owner: &str, repo: &str, asset: &str) -> String {
    format!("https://gitee.com/{owner}/{repo}/releases/download/{GITEE_RELEASE_TAG}/{asset}")
}

fn route_rules(
    config: &Config,
    server_names: &[String],
    route_profile: &str,
    tun_profile: &str,
) -> YaoeResult<Vec<RouteRule>> {
    let mut rules = Vec::new();
    let netbird_direct_action = netbird_direct_action(tun_profile);
    if route_profile == "service" {
        rules.push(RouteRule::ProcessName(ProcessNameRouteRule {
            process_name: NETBIRD_PROCESS_NAMES.to_vec(),
            action: netbird_direct_action,
            outbound: "direct",
        }));
    }
    rules.push(RouteRule::DnsHijack(DnsHijackRouteRule {
        port: DNS_HIJACK_PORT,
        action: "hijack-dns",
    }));
    rules.push(RouteRule::Sniff(SniffRouteRule { action: "sniff" }));
    rules.push(RouteRule::Domain(DomainRouteRule {
        domain: NETBIRD_DOMAIN_EXACT.to_vec(),
        action: netbird_direct_action,
        outbound: "direct",
    }));
    rules.push(RouteRule::DomainSuffix(DomainSuffixRouteRule {
        domain_suffix: NETBIRD_DOMAIN_SUFFIX.to_vec(),
        action: netbird_direct_action,
        outbound: "direct",
    }));
    rules.push(RouteRule::Cidr(CidrRouteRule {
        ip_cidr: direct_cidrs(config, server_names)?,
        action: netbird_direct_action,
        outbound: "direct",
    }));
    rules.push(RouteRule::Ipv6Reject(Ipv6RejectRouteRule {
        ip_version: 6,
        action: "reject",
        method: "default",
        no_drop: PUBLIC_IPV6_DENIAL_NO_DROP,
    }));
    rules.push(RouteRule::RuleSet(RuleSetRouteRule {
        rule_set: vec![CN_DOMAIN_RULE_TAG, CN_IPV4_RULE_TAG],
        action: "route",
        outbound: "direct",
    }));
    Ok(rules)
}

fn netbird_direct_action(tun_profile: &str) -> &'static str {
    if tun_profile == "linux-service" {
        "bypass"
    } else {
        "route"
    }
}

fn direct_ipv4_cidrs(config: &Config, server_names: &[String]) -> YaoeResult<Vec<String>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for cidr in BUILTIN_DIRECT_IPV4_CIDRS {
        push_cidr(&mut out, &mut seen, cidr)?;
    }
    push_cidr(&mut out, &mut seen, NETBIRD_DIRECT_CIDR)?;
    for cidr in &config.route.direct_cidrs {
        push_cidr(&mut out, &mut seen, cidr)?;
    }
    for name in server_names {
        let ip: Ipv4Addr = config.server[name]
            .ip
            .parse()
            .map_err(|e| YaoeError::Internal(format!("egress IPv4 parse after validation: {e}")))?;
        push_cidr(&mut out, &mut seen, &format!("{ip}/32"))?;
    }
    Ok(out)
}

fn direct_cidrs(config: &Config, server_names: &[String]) -> YaoeResult<Vec<String>> {
    let mut out = direct_ipv4_cidrs(config, server_names)?;
    let mut seen: HashSet<String> = out.iter().cloned().collect();
    for cidr in BUILTIN_DIRECT_IPV6_CIDRS {
        push_cidr(&mut out, &mut seen, cidr)?;
    }
    Ok(out)
}

fn push_cidr(out: &mut Vec<String>, seen: &mut HashSet<String>, cidr: &str) -> YaoeResult<()> {
    let parsed: IpNet = cidr
        .parse()
        .map_err(|e| YaoeError::Internal(format!("CIDR parse after validation: {e}")))?;
    let canonical = parsed.trunc().to_string();
    if seen.insert(canonical.clone()) {
        out.push(canonical);
    }
    Ok(())
}

pub fn validate_client_semantics(config: &Value, platform: ClientPlatform) -> YaoeResult<()> {
    reject_forbidden_keys(config, platform)?;
    let auto_redirect_count = count_key(config, "auto_redirect");
    match platform {
        platform if platform.is_linux() && auto_redirect_count != 1 => {
            return Err(YaoeError::Internal(
                "Linux generated config must contain exactly one auto_redirect field".into(),
            ));
        }
        ClientPlatform::MacosAmd64 | ClientPlatform::MacosArm64 if auto_redirect_count != 0 => {
            return Err(YaoeError::Internal(
                "non-Linux generated config contains auto_redirect".into(),
            ));
        }
        ClientPlatform::Ios | ClientPlatform::Android if auto_redirect_count != 0 => {
            return Err(YaoeError::Internal(
                "mobile generated config contains auto_redirect".into(),
            ));
        }
        _ => {}
    }
    let strict_route_count = count_key(config, "strict_route");
    match platform {
        platform if platform.is_service() && strict_route_count != 1 => {
            return Err(YaoeError::Internal(
                "service generated config must contain exactly one strict_route field".into(),
            ));
        }
        ClientPlatform::Ios | ClientPlatform::Android if strict_route_count != 0 => {
            return Err(YaoeError::Internal(
                "mobile generated config contains strict_route".into(),
            ));
        }
        _ => {}
    }
    let auto_detect_count = count_key(config, "auto_detect_interface");
    match platform {
        platform if platform.is_service() && auto_detect_count != 1 => {
            return Err(YaoeError::Internal(
                "service generated config must contain exactly one auto_detect_interface field"
                    .into(),
            ));
        }
        ClientPlatform::Ios | ClientPlatform::Android if auto_detect_count != 0 => {
            return Err(YaoeError::Internal(
                "mobile generated config contains auto_detect_interface".into(),
            ));
        }
        _ => {}
    }
    if config.get("experimental").is_some() {
        return Err(YaoeError::Internal(
            "generated client config contains experimental".into(),
        ));
    }
    validate_shared_client_shape(config, platform)?;
    validate_ipv6_containment(config)?;
    let rule_sets = config
        .get("route")
        .and_then(|route| route.get("rule_set"))
        .and_then(Value::as_array)
        .ok_or_else(|| YaoeError::Internal("generated client config missing rule_set".into()))?;
    if rule_sets
        .iter()
        .map(|rule_set| rule_set.get("tag").and_then(Value::as_str))
        .collect::<Vec<_>>()
        != vec![Some(CN_DOMAIN_RULE_TAG), Some(CN_IPV4_RULE_TAG)]
    {
        return Err(YaoeError::Internal(
            "generated client rule-set tags are not cn-domain then cn-ipv4".into(),
        ));
    }
    for rule_set in rule_sets {
        if rule_set.get("type").and_then(Value::as_str) != Some("remote")
            || rule_set.get("format").and_then(Value::as_str) != Some("binary")
            || rule_set.get("path").is_some()
            || rule_set.get("download_detour").and_then(Value::as_str) != Some("direct")
            || rule_set.get("update_interval").and_then(Value::as_str) != Some("1d")
        {
            return Err(YaoeError::Internal(
                "generated client rule-set is not remote binary over direct detour".into(),
            ));
        }
    }
    Ok(())
}

fn validate_shared_client_shape(config: &Value, platform: ClientPlatform) -> YaoeResult<()> {
    let dns = config
        .get("dns")
        .ok_or_else(|| YaoeError::Internal("generated client config missing DNS".into()))?;
    let dns_servers = dns
        .get("servers")
        .and_then(Value::as_array)
        .ok_or_else(|| YaoeError::Internal("generated client DNS missing servers".into()))?;
    if dns_servers
        .iter()
        .map(|server| server.get("tag").and_then(Value::as_str))
        .collect::<Vec<_>>()
        != vec![Some("cn-dns"), Some("remote-dns")]
    {
        return Err(YaoeError::Internal(
            "generated client DNS servers are not cn-dns then remote-dns".into(),
        ));
    }
    let cn_dns = &dns_servers[0];
    if cn_dns.get("type").and_then(Value::as_str) != Some(CN_DNS_TYPE)
        || cn_dns.get("server").and_then(Value::as_str) != Some(CN_DNS_SERVER)
        || cn_dns.get("server_port").and_then(Value::as_u64) != Some(CN_DNS_PORT.into())
        || cn_dns.get("path").and_then(Value::as_str) != Some(CN_DNS_PATH)
        || cn_dns.get("detour").is_some()
        || cn_dns
            .get("tls")
            .and_then(|tls| tls.get("server_name"))
            .and_then(Value::as_str)
            != Some(CN_DNS_TLS_SERVER_NAME)
    {
        return Err(YaoeError::Internal(
            "generated client cn-dns server does not match contract".into(),
        ));
    }
    let remote_dns = &dns_servers[1];
    if remote_dns.get("type").and_then(Value::as_str) != Some(REMOTE_DNS_TYPE)
        || remote_dns.get("server").and_then(Value::as_str) != Some(REMOTE_DNS_SERVER)
        || remote_dns.get("server_port").and_then(Value::as_u64) != Some(REMOTE_DNS_PORT.into())
        || remote_dns.get("path").and_then(Value::as_str) != Some(REMOTE_DNS_PATH)
        || remote_dns.get("detour").and_then(Value::as_str) != Some("proxy")
        || remote_dns
            .get("tls")
            .and_then(|tls| tls.get("server_name"))
            .and_then(Value::as_str)
            != Some(REMOTE_DNS_TLS_SERVER_NAME)
    {
        return Err(YaoeError::Internal(
            "generated client remote-dns server does not match contract".into(),
        ));
    }
    let dns_rules = dns
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| YaoeError::Internal("generated client DNS missing rules".into()))?;
    if dns_rules.len() != 3
        || dns_rules[0].get("domain") != Some(&serde_json::json!(NETBIRD_DOMAIN_EXACT))
        || dns_rules[0].get("action").and_then(Value::as_str) != Some("route")
        || dns_rules[0].get("server").and_then(Value::as_str) != Some("cn-dns")
        || dns_rules[1].get("domain_suffix") != Some(&serde_json::json!(NETBIRD_DOMAIN_SUFFIX))
        || dns_rules[1].get("action").and_then(Value::as_str) != Some("route")
        || dns_rules[1].get("server").and_then(Value::as_str) != Some("cn-dns")
        || dns_rules[2].get("rule_set") != Some(&serde_json::json!([CN_DOMAIN_RULE_TAG]))
        || dns_rules[2].get("action").and_then(Value::as_str) != Some("route")
        || dns_rules[2].get("server").and_then(Value::as_str) != Some("cn-dns")
        || count_key(&Value::Array(dns_rules.clone()), "strategy") != 0
    {
        return Err(YaoeError::Internal(
            "generated client DNS rules do not match contract".into(),
        ));
    }
    if dns.get("final").and_then(Value::as_str) != Some("remote-dns")
        || dns.get("strategy").and_then(Value::as_str) != Some(DNS_STRATEGY)
        || dns.get("reverse_mapping").and_then(Value::as_bool) != Some(true)
    {
        return Err(YaoeError::Internal(
            "generated client DNS final strategy does not match contract".into(),
        ));
    }

    let outbounds = config
        .get("outbounds")
        .and_then(Value::as_array)
        .ok_or_else(|| YaoeError::Internal("generated client config missing outbounds".into()))?;
    let Some(proxy) = outbounds.first() else {
        return Err(YaoeError::Internal(
            "generated client outbounds missing proxy".into(),
        ));
    };
    if proxy.get("type").and_then(Value::as_str) != Some("urltest")
        || proxy.get("tag").and_then(Value::as_str) != Some("proxy")
        || proxy
            .get("outbounds")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        || proxy.get("url").is_some()
        || proxy.get("interval").is_some()
        || proxy.get("tolerance").is_some()
        || proxy.get("idle_timeout").is_some()
    {
        return Err(YaoeError::Internal(
            "generated client proxy URLTest outbound does not match contract".into(),
        ));
    }
    let mut saw_direct = false;
    for outbound in outbounds {
        match outbound.get("type").and_then(Value::as_str) {
            Some("vless") => {
                let server = outbound
                    .get("server")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        YaoeError::Internal("generated VLESS outbound missing server".into())
                    })?;
                server.parse::<Ipv4Addr>().map_err(|_| {
                    YaoeError::Internal(
                        "generated VLESS outbound server must be an IPv4 literal".into(),
                    )
                })?;
                if outbound.get("network").is_some()
                    || outbound.get("flow").and_then(Value::as_str) != Some("xtls-rprx-vision")
                    || outbound
                        .get("tls")
                        .and_then(|tls| tls.get("enabled"))
                        .and_then(Value::as_bool)
                        != Some(true)
                    || outbound
                        .get("tls")
                        .and_then(|tls| tls.get("utls"))
                        .and_then(|utls| utls.get("fingerprint"))
                        .and_then(Value::as_str)
                        != Some("chrome")
                    || outbound
                        .get("tls")
                        .and_then(|tls| tls.get("reality"))
                        .and_then(|reality| reality.get("enabled"))
                        .and_then(Value::as_bool)
                        != Some(true)
                {
                    return Err(YaoeError::Internal(
                        "generated VLESS outbound does not match Reality/Vision contract".into(),
                    ));
                }
            }
            Some("direct") if outbound.get("tag").and_then(Value::as_str) == Some("direct") => {
                saw_direct = true;
            }
            Some("urltest") => {}
            _ => {
                return Err(YaoeError::Internal(
                    "generated client config contains unsupported outbound".into(),
                ));
            }
        }
    }
    if !saw_direct {
        return Err(YaoeError::Internal(
            "generated client config missing direct outbound".into(),
        ));
    }

    let route = config
        .get("route")
        .ok_or_else(|| YaoeError::Internal("generated client config missing route".into()))?;
    if route.get("default_domain_resolver").and_then(Value::as_str) != Some("remote-dns")
        || route.get("final").and_then(Value::as_str) != Some("proxy")
    {
        return Err(YaoeError::Internal(
            "generated client route final resolver does not match contract".into(),
        ));
    }
    let rules = route
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| YaoeError::Internal("generated client route missing rules".into()))?;
    let mut offset = 0;
    let netbird_direct_action = if platform.is_linux() {
        "bypass"
    } else {
        "route"
    };
    if rules
        .first()
        .and_then(|rule| rule.get("process_name"))
        .is_some()
    {
        if rules[0].get("process_name") != Some(&serde_json::json!(NETBIRD_PROCESS_NAMES))
            || rules[0].get("action").and_then(Value::as_str) != Some(netbird_direct_action)
            || rules[0].get("outbound").and_then(Value::as_str) != Some("direct")
        {
            return Err(YaoeError::Internal(
                "generated client NetBird process route rule does not match contract".into(),
            ));
        }
        offset = 1;
    }
    if rules.len() != offset + 7
        || rules[offset].get("port").and_then(Value::as_u64) != Some(DNS_HIJACK_PORT.into())
        || rules[offset].get("action").and_then(Value::as_str) != Some("hijack-dns")
        || rules[offset + 1].get("action").and_then(Value::as_str) != Some("sniff")
        || rules[offset + 2].get("domain") != Some(&serde_json::json!(NETBIRD_DOMAIN_EXACT))
        || rules[offset + 2].get("action").and_then(Value::as_str) != Some(netbird_direct_action)
        || rules[offset + 2].get("outbound").and_then(Value::as_str) != Some("direct")
        || rules[offset + 3].get("domain_suffix") != Some(&serde_json::json!(NETBIRD_DOMAIN_SUFFIX))
        || rules[offset + 3].get("action").and_then(Value::as_str) != Some(netbird_direct_action)
        || rules[offset + 3].get("outbound").and_then(Value::as_str) != Some("direct")
        || rules[offset + 4]
            .get("ip_cidr")
            .and_then(Value::as_array)
            .is_none()
        || rules[offset + 4].get("action").and_then(Value::as_str) != Some(netbird_direct_action)
        || rules[offset + 4].get("outbound").and_then(Value::as_str) != Some("direct")
        || rules[offset + 6].get("rule_set")
            != Some(&serde_json::json!([CN_DOMAIN_RULE_TAG, CN_IPV4_RULE_TAG]))
        || rules[offset + 6].get("action").and_then(Value::as_str) != Some("route")
        || rules[offset + 6].get("outbound").and_then(Value::as_str) != Some("direct")
    {
        return Err(YaoeError::Internal(
            "generated client route rules do not match contract order".into(),
        ));
    }
    let route_cidrs = rules[offset + 4]
        .get("ip_cidr")
        .and_then(Value::as_array)
        .ok_or_else(|| YaoeError::Internal("generated client route missing CIDRs".into()))?;
    let tun_cidrs = config
        .get("inbounds")
        .and_then(Value::as_array)
        .and_then(|inbounds| inbounds.first())
        .and_then(|inbound| inbound.get("route_exclude_address"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            YaoeError::Internal("generated client TUN missing route_exclude_address".into())
        })?;
    if route_cidrs != tun_cidrs {
        return Err(YaoeError::Internal(
            "generated client route CIDRs and TUN exclusions differ".into(),
        ));
    }
    Ok(())
}

fn validate_ipv6_containment(config: &Value) -> YaoeResult<()> {
    let address = config
        .get("inbounds")
        .and_then(Value::as_array)
        .and_then(|inbounds| inbounds.first())
        .and_then(|inbound| inbound.get("address"))
        .ok_or_else(|| YaoeError::Internal("generated client config missing TUN address".into()))?;
    if address != &serde_json::json!([TUN_IPV4_ADDRESS, TUN_IPV6_ADDRESS]) {
        return Err(YaoeError::Internal(
            "generated client config must contain IPv4 and IPv6 TUN addresses".into(),
        ));
    }
    if config
        .get("dns")
        .and_then(|dns| dns.get("strategy"))
        .and_then(Value::as_str)
        != Some(DNS_STRATEGY)
    {
        return Err(YaoeError::Internal(
            "generated client config DNS strategy must be ipv4_only".into(),
        ));
    }
    let rules = config
        .get("route")
        .and_then(|route| route.get("rules"))
        .and_then(Value::as_array)
        .ok_or_else(|| YaoeError::Internal("generated client config missing route rules".into()))?;
    let ipv6_reject_pos = rules.iter().position(|rule| {
        rule.get("ip_version").and_then(Value::as_u64) == Some(6)
            && rule.get("action").and_then(Value::as_str) == Some("reject")
            && rule.get("method").and_then(Value::as_str) == Some("default")
            && rule.get("no_drop").and_then(Value::as_bool) == Some(PUBLIC_IPV6_DENIAL_NO_DROP)
    });
    let china_direct_pos = rules.iter().position(|rule| {
        rule.get("rule_set")
            .and_then(Value::as_array)
            .is_some_and(|sets| {
                sets.as_slice()
                    == [
                        Value::String(CN_DOMAIN_RULE_TAG.to_string()),
                        Value::String(CN_IPV4_RULE_TAG.to_string()),
                    ]
            })
    });
    match (ipv6_reject_pos, china_direct_pos) {
        (Some(reject), Some(china)) if reject < china => Ok(()),
        _ => Err(YaoeError::Internal(
            "generated client config must reject IPv6 before China-direct rule sets".into(),
        )),
    }
}

fn reject_forbidden_keys(value: &Value, platform: ClientPlatform) -> YaoeResult<()> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if matches!(
                    key.as_str(),
                    "network"
                        | "dns_mode"
                        | "dns_address"
                        | "interval"
                        | "tolerance"
                        | "idle_timeout"
                        | "cache_file"
                        | "override_android_vpn"
                        | "default_interface"
                        | "default_mark"
                ) {
                    return Err(YaoeError::Internal(format!(
                        "generated client config contains forbidden key: {key}"
                    )));
                }
                if key == "auto_redirect" {
                    match platform {
                        platform if platform.is_linux() && child.as_bool() == Some(true) => {}
                        platform if platform.is_linux() => {
                            return Err(YaoeError::Internal(
                                "Linux generated config auto_redirect must be true".into(),
                            ));
                        }
                        _ => {
                            return Err(YaoeError::Internal(
                                "non-Linux generated config contains auto_redirect".into(),
                            ));
                        }
                    }
                }
                if platform.is_mobile()
                    && matches!(
                        key.as_str(),
                        "strict_route"
                            | "auto_redirect_input_mark"
                            | "auto_redirect_output_mark"
                            | "auto_redirect_reset_mark"
                            | "auto_redirect_nfqueue"
                            | "auto_redirect_iproute2_fallback_rule_index"
                            | "exclude_mptcp"
                            | "interface_name"
                            | "gso"
                            | "stack"
                            | "route_address"
                            | "route_address_set"
                            | "route_exclude_address_set"
                            | "include_interface"
                            | "exclude_interface"
                            | "include_uid"
                            | "exclude_uid"
                            | "include_uid_range"
                            | "exclude_uid_range"
                            | "include_android_user"
                            | "include_package"
                            | "exclude_package"
                            | "platform"
                            | "auto_detect_interface"
                            | "override_android_vpn"
                            | "default_interface"
                            | "default_mark"
                    )
                {
                    return Err(YaoeError::Internal(format!(
                        "mobile generated config contains forbidden key: {key}"
                    )));
                }
                reject_forbidden_keys(child, platform)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_forbidden_keys(child, platform)?;
            }
        }
        Value::String(text)
            if text.contains("BEGIN PRIVATE KEY")
                || text.contains("BEGIN CERTIFICATE")
                || text.contains("OPENSSH") =>
        {
            return Err(YaoeError::Internal(
                "generated client config contains forbidden secret material".into(),
            ));
        }
        Value::String(_) => {}
        _ => {}
    }
    Ok(())
}

fn count_key(value: &Value, key: &str) -> usize {
    match value {
        Value::Object(map) => {
            map.iter()
                .filter(|(candidate, _)| *candidate == key)
                .count()
                + map
                    .values()
                    .map(|child| count_key(child, key))
                    .sum::<usize>()
        }
        Value::Array(values) => values.iter().map(|child| count_key(child, key)).sum(),
        _ => 0,
    }
}
