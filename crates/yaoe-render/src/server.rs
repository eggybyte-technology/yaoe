use serde::Serialize;
use yaoe_home::{YaoeError, YaoeResult};

#[derive(Debug, Clone)]
pub struct ServerRenderInput {
    pub server_name: String,
    pub endpoint_ip: String,
    pub port: u16,
    pub vless_uuid: String,
    pub reality_private_key: String,
    pub reality_short_id: String,
    pub handshake_server: String,
    pub handshake_port: u16,
}

#[derive(Debug, Clone)]
pub struct HealthProbeRenderInput {
    pub endpoint_ip: String,
    pub port: u16,
    pub probe_port: u16,
    pub vless_uuid: String,
    pub reality_public_key: String,
    pub reality_short_id: String,
    pub handshake_server: String,
}

pub fn render_server_config(input: &ServerRenderInput) -> YaoeResult<String> {
    let _ip: std::net::Ipv4Addr = input
        .endpoint_ip
        .parse()
        .map_err(|e| YaoeError::Internal(format!("server render IPv4 parse: {e}")))?;
    let doc = ServerConfigDoc {
        log: Log {
            level: "info",
            output: format!("/var/log/yaoe/{}.log", input.server_name),
        },
        inbounds: vec![VlessInbound {
            kind: "vless",
            tag: "vless-in",
            listen: "0.0.0.0",
            listen_port: input.port,
            users: vec![VlessUser {
                uuid: input.vless_uuid.clone(),
                flow: "xtls-rprx-vision",
            }],
            tls: Tls {
                enabled: true,
                server_name: input.handshake_server.clone(),
                reality: Reality {
                    enabled: true,
                    handshake: Handshake {
                        server: input.handshake_server.clone(),
                        server_port: input.handshake_port,
                    },
                    private_key: input.reality_private_key.clone(),
                    short_id: vec![input.reality_short_id.clone()],
                },
            },
        }],
        outbounds: vec![DirectOutbound {
            kind: "direct",
            tag: "direct",
        }],
        route: Route {
            final_out: "direct",
        },
    };
    let mut text = serde_json::to_string_pretty(&doc)
        .map_err(|e| YaoeError::Internal(format!("render server config: {e}")))?;
    text.push('\n');
    validate_server_config(&text)?;
    Ok(text)
}

pub fn render_health_probe_config(input: &HealthProbeRenderInput) -> YaoeResult<String> {
    let _ip: std::net::Ipv4Addr = input
        .endpoint_ip
        .parse()
        .map_err(|e| YaoeError::Internal(format!("health probe IPv4 parse: {e}")))?;
    let doc = HealthProbeConfigDoc {
        log: ProbeLog { level: "debug" },
        inbounds: vec![MixedInbound {
            kind: "mixed",
            tag: "mixed-in",
            listen: yaoe_home::HEALTH_PROBE_BIND_HOST,
            listen_port: input.probe_port,
            set_system_proxy: false,
        }],
        outbounds: vec![
            ProbeOutbound::Vless(ProbeVlessOutbound {
                kind: "vless",
                tag: "probe",
                server: input.endpoint_ip.clone(),
                server_port: input.port,
                uuid: input.vless_uuid.clone(),
                flow: "xtls-rprx-vision",
                tls: ProbeVlessTls {
                    enabled: true,
                    server_name: input.handshake_server.clone(),
                    utls: ProbeUtls {
                        enabled: true,
                        fingerprint: "chrome",
                    },
                    reality: ProbeRealityTls {
                        enabled: true,
                        public_key: input.reality_public_key.clone(),
                        short_id: input.reality_short_id.clone(),
                    },
                },
            }),
            ProbeOutbound::Direct(ProbeDirectOutbound {
                kind: "direct",
                tag: "direct",
            }),
        ],
        route: ProbeRoute { final_out: "probe" },
    };
    let mut text = serde_json::to_string_pretty(&doc)
        .map_err(|e| YaoeError::Internal(format!("render health probe config: {e}")))?;
    text.push('\n');
    validate_health_probe_config(&text)?;
    Ok(text)
}

fn validate_server_config(config: &str) -> YaoeResult<()> {
    for forbidden in [
        "certificate",
        "certificate_path",
        "key_path",
        "acme",
        "certificate_provider",
    ] {
        if config.contains(forbidden) {
            return Err(YaoeError::Internal(format!(
                "generated server config contains forbidden field: {forbidden}"
            )));
        }
    }
    Ok(())
}

fn validate_health_probe_config(config: &str) -> YaoeResult<()> {
    for forbidden in [
        "\"dns\"",
        "\"tun\"",
        "\"rule_set\"",
        "\"urltest\"",
        "auto_route",
        "strict_route",
        "auto_redirect",
    ] {
        if config.contains(forbidden) {
            return Err(YaoeError::Internal(format!(
                "generated health probe config contains forbidden field: {forbidden}"
            )));
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct ServerConfigDoc {
    log: Log,
    inbounds: Vec<VlessInbound>,
    outbounds: Vec<DirectOutbound>,
    route: Route,
}

#[derive(Serialize)]
struct Log {
    level: &'static str,
    output: String,
}

#[derive(Serialize)]
struct VlessInbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
    listen: &'static str,
    listen_port: u16,
    users: Vec<VlessUser>,
    tls: Tls,
}

#[derive(Serialize)]
struct VlessUser {
    uuid: String,
    flow: &'static str,
}

#[derive(Serialize)]
struct Tls {
    enabled: bool,
    server_name: String,
    reality: Reality,
}

#[derive(Serialize)]
struct Reality {
    enabled: bool,
    handshake: Handshake,
    private_key: String,
    short_id: Vec<String>,
}

#[derive(Serialize)]
struct Handshake {
    server: String,
    server_port: u16,
}

#[derive(Serialize)]
struct DirectOutbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
}

#[derive(Serialize)]
struct Route {
    #[serde(rename = "final")]
    final_out: &'static str,
}

#[derive(Serialize)]
struct HealthProbeConfigDoc {
    log: ProbeLog,
    inbounds: Vec<MixedInbound>,
    outbounds: Vec<ProbeOutbound>,
    route: ProbeRoute,
}

#[derive(Serialize)]
struct ProbeLog {
    level: &'static str,
}

#[derive(Serialize)]
struct MixedInbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
    listen: &'static str,
    listen_port: u16,
    set_system_proxy: bool,
}

#[derive(Serialize)]
#[serde(untagged)]
enum ProbeOutbound {
    Vless(ProbeVlessOutbound),
    Direct(ProbeDirectOutbound),
}

#[derive(Serialize)]
struct ProbeVlessOutbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
    server: String,
    server_port: u16,
    uuid: String,
    flow: &'static str,
    tls: ProbeVlessTls,
}

#[derive(Serialize)]
struct ProbeVlessTls {
    enabled: bool,
    server_name: String,
    utls: ProbeUtls,
    reality: ProbeRealityTls,
}

#[derive(Serialize)]
struct ProbeUtls {
    enabled: bool,
    fingerprint: &'static str,
}

#[derive(Serialize)]
struct ProbeRealityTls {
    enabled: bool,
    public_key: String,
    short_id: String,
}

#[derive(Serialize)]
struct ProbeDirectOutbound {
    #[serde(rename = "type")]
    kind: &'static str,
    tag: &'static str,
}

#[derive(Serialize)]
struct ProbeRoute {
    #[serde(rename = "final")]
    final_out: &'static str,
}
