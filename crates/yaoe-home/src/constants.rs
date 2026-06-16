pub const YAOE_PRODUCT_REVISION: &str = "v0.0.1";
pub const RUST_TOOLCHAIN_VERSION: &str = "1.96.0";
pub const SING_BOX_VERSION: &str = "1.13.13";
pub const SING_BOX_RELEASE_TAG: &str = "v1.13.13";
pub const SING_BOX_ARTIFACT_ROOT: &str = "sing-box/1.13.13";
pub const MIHOMO_VALIDATION_VERSION: &str = "1.19.27";
pub const CLASH_VERGE_REV_REFERENCE_VERSION: &str = "2.5.1";
pub const CLASH_VERGE_REV_REFERENCE_TAG: &str = "v2.5.1";
pub const CLASH_VERGE_REV_REFERENCE_DATE: &str = "2026-05-20";
pub const CLASH_VERGE_MIHOMO_BASELINE_VERSION: &str = "1.19.25";
pub const GITEE_BOOTSTRAP_BRANCH: &str = "main";
pub const GITEE_RELEASE_TAG: &str = "yaoe-v0.0.1-sing-box-1.13.13";
pub const SERVICE_SCRIPT_TARGETS: [&str; 2] = ["linux", "macos"];
pub const SERVICE_CONFIG_VARIANTS: [&str; 4] =
    ["linux-amd64", "linux-arm64", "macos-amd64", "macos-arm64"];
pub const MOBILE_CONFIG_VARIANTS: [&str; 2] = ["ios", "android"];
pub const GUI_CONFIG_VARIANTS: [&str; 1] = ["clash-verge"];
pub const CONFIG_VARIANTS: [&str; 7] = [
    "clash-verge",
    "linux-amd64",
    "linux-arm64",
    "macos-amd64",
    "macos-arm64",
    "ios",
    "android",
];
pub const MANAGED_SERVER_RUNTIME_VARIANTS: [&str; 2] = ["linux-amd64", "linux-arm64"];
pub const CONFIG_KEY_RANDOM_BYTES: usize = 96;
pub const CONFIG_KEY_LENGTH: usize = 128;
pub const REALITY_PRIVATE_KEY_LENGTH: usize = 43;
pub const REALITY_PUBLIC_KEY_LENGTH: usize = 43;
pub const REALITY_SHORT_ID_BYTES: usize = 8;
pub const REALITY_SHORT_ID_HEX_LENGTH: usize = 16;
pub const SERVER_PORT_MIN: u16 = 20000;
pub const SERVER_PORT_MAX: u16 = 60999;
pub const R2_JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";
pub const R2_YAML_CONTENT_TYPE: &str = "text/yaml; charset=utf-8";
pub const R2_CONFIG_CACHE_CONTROL: &str = "no-store";
pub const R2_CUSTOM_DOMAIN_MIN_TLS: &str = "1.2";
pub const CLOUDFLARE_PUBLIC_FETCH_ATTEMPTS: usize = 60;
pub const CLOUDFLARE_PUBLIC_FETCH_INTERVAL_SECONDS: u64 = 5;
pub const HEALTH_PROBE_URL: &str = "https://www.gstatic.com/generate_204";
pub const HEALTH_PROBE_EXPECTED_STATUS: u16 = 204;
pub const HEALTH_PROBE_CURL_PROXY_KIND: &str = "socks5-remote-resolve";
pub const HEALTH_PROBE_BIND_HOST: &str = "127.0.0.1";
pub const TUN_IPV4_ADDRESS: &str = "172.19.0.1/30";
pub const PUBLIC_IPV6_DENIAL_NO_DROP: bool = true;
pub const IPV6_REJECT_NO_DROP: bool = PUBLIC_IPV6_DENIAL_NO_DROP;
pub const NETBIRD_DIRECT_CIDR: &str = "100.64.0.0/10";
pub const NETBIRD_PROCESS_NAMES: [&str; 6] = [
    "netbird.exe",
    "NetBird.exe",
    "netbird",
    "NetBird",
    "netbird-ui",
    "NetBird UI",
];
pub const NETBIRD_DOMAIN_EXACT: [&str; 4] = [
    "api.netbird.io",
    "signal.netbird.io",
    "stun.netbird.io",
    "turn.netbird.io",
];
pub const NETBIRD_DOMAIN_SUFFIX: [&str; 3] = ["netbird.io", "netbird.cloud", "relay.netbird.io"];
pub const NETBIRD_MIHOMO_FAKE_IP_FILTER: [&str; 12] = [
    "netbird.io",
    "*.netbird.io",
    "netbird.cloud",
    "*.netbird.cloud",
    "api.netbird.io",
    "signal.netbird.io",
    "stun.netbird.io",
    "turn.netbird.io",
    "*.relay.netbird.io",
    "*.lan",
    "*.local",
    "localhost.ptlogin2.qq.com",
];
pub const BUILTIN_DIRECT_IPV4_CIDRS: [&str; 6] = [
    "127.0.0.0/8",
    "169.254.0.0/16",
    "224.0.0.0/4",
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
];
pub const SING_BOX_DNS_STRATEGY: &str = "ipv4_only";
pub const SING_BOX_DNS_HIJACK_PORT: u16 = 53;
pub const SING_BOX_CN_DNS_TYPE: &str = "https";
pub const SING_BOX_CN_DNS_SERVER: &str = "223.5.5.5";
pub const SING_BOX_CN_DNS_PORT: u16 = 443;
pub const SING_BOX_CN_DNS_PATH: &str = "/dns-query";
pub const SING_BOX_CN_DNS_TLS_SERVER_NAME: &str = "dns.alidns.com";
pub const SING_BOX_REMOTE_DNS_TYPE: &str = "https";
pub const SING_BOX_REMOTE_DNS_SERVER: &str = "1.1.1.1";
pub const SING_BOX_REMOTE_DNS_PORT: u16 = 443;
pub const SING_BOX_REMOTE_DNS_PATH: &str = "/dns-query";
pub const SING_BOX_REMOTE_DNS_TLS_SERVER_NAME: &str = "cloudflare-dns.com";
pub const DNS_STRATEGY: &str = SING_BOX_DNS_STRATEGY;
pub const DNS_HIJACK_PORT: u16 = SING_BOX_DNS_HIJACK_PORT;
pub const CN_DNS_TYPE: &str = SING_BOX_CN_DNS_TYPE;
pub const CN_DNS_SERVER: &str = SING_BOX_CN_DNS_SERVER;
pub const CN_DNS_PORT: u16 = SING_BOX_CN_DNS_PORT;
pub const CN_DNS_PATH: &str = SING_BOX_CN_DNS_PATH;
pub const CN_DNS_TLS_SERVER_NAME: &str = SING_BOX_CN_DNS_TLS_SERVER_NAME;
pub const REMOTE_DNS_TYPE: &str = SING_BOX_REMOTE_DNS_TYPE;
pub const REMOTE_DNS_SERVER: &str = SING_BOX_REMOTE_DNS_SERVER;
pub const REMOTE_DNS_PORT: u16 = SING_BOX_REMOTE_DNS_PORT;
pub const REMOTE_DNS_PATH: &str = SING_BOX_REMOTE_DNS_PATH;
pub const REMOTE_DNS_TLS_SERVER_NAME: &str = SING_BOX_REMOTE_DNS_TLS_SERVER_NAME;
pub const HEALTH_PROBE_STARTUP_TIMEOUT_SECONDS: u64 = 3;
pub const HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS: u64 = 8;
pub const HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS: u64 = 12;
pub const HEALTH_PROBE_PORT_RETRY_LIMIT: usize = 3;

pub const CN_DOMAIN_RULE_TAG: &str = "cn-domain";
pub const CN_IPV4_RULE_TAG: &str = "cn-ipv4";
pub const CN_DOMAIN_PUBLIC_ASSET: &str = "cn-domain.srs";
pub const CN_IPV4_PUBLIC_ASSET: &str = "cn-ipv4.srs";
pub const CN_DOMAIN_UPSTREAM_URL: &str = "https://cdn.jsdelivr.net/gh/Dreista/sing-box-rule-set-cn@rule-set/accelerated-domains.china.conf.srs";
pub const CN_IPV4_UPSTREAM_URL: &str =
    "https://cdn.jsdelivr.net/gh/Dreista/sing-box-rule-set-cn@rule-set/chnroutes.txt.srs";
pub const MIHOMO_PROFILE_FILE: &str = "clash-verge.yaml";
pub const MIHOMO_MIXED_PORT: u16 = 7890;
pub const MIHOMO_MODE: &str = "rule";
pub const MIHOMO_LOG_LEVEL: &str = "info";
pub const MIHOMO_IPV6: bool = false;
pub const MIHOMO_ALLOW_LAN: bool = false;
pub const MIHOMO_DNS_ENABLE: bool = true;
pub const MIHOMO_DNS_IPV6: bool = false;
pub const MIHOMO_DNS_ENHANCED_MODE: &str = "fake-ip";
pub const MIHOMO_FAKE_IP_RANGE: &str = "198.18.0.1/16";
pub const MIHOMO_TUN_ENABLE: bool = true;
pub const MIHOMO_TUN_STACK: &str = "mixed";
pub const MIHOMO_TUN_AUTO_ROUTE: bool = true;
pub const MIHOMO_TUN_AUTO_DETECT_INTERFACE: bool = true;
pub const MIHOMO_TUN_STRICT_ROUTE: bool = true;
pub const MIHOMO_TUN_DNS_HIJACK: [&str; 2] = ["any:53", "tcp://any:53"];
pub const MIHOMO_URL_TEST_URL: &str = "https://www.gstatic.com/generate_204";
pub const MIHOMO_URL_TEST_INTERVAL_SECONDS: u16 = 300;
pub const MIHOMO_GEODATA_MODE: bool = true;
pub const MIHOMO_GEO_AUTO_UPDATE: bool = true;
pub const MIHOMO_GEO_UPDATE_INTERVAL_HOURS: u16 = 24;
pub const MIHOMO_GEOIP_URL: &str =
    "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat";
pub const MIHOMO_GEOSITE_URL: &str =
    "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat";
pub const MIHOMO_MMDB_URL: &str =
    "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/country.mmdb";
pub const MIHOMO_DEFAULT_NAMESERVER: [&str; 2] = ["223.5.5.5", "119.29.29.29"];
pub const MIHOMO_NAMESERVER: [&str; 2] = [
    "https://dns.alidns.com/dns-query",
    "https://doh.pub/dns-query",
];
pub const MIHOMO_DIRECT_NAMESERVER: [&str; 2] = [
    "https://dns.alidns.com/dns-query",
    "https://doh.pub/dns-query",
];
pub const MIHOMO_FALLBACK: [&str; 2] = [
    "https://1.1.1.1/dns-query#PROXY",
    "https://8.8.8.8/dns-query#PROXY",
];
pub const MIHOMO_FALLBACK_FILTER_GEOIP: bool = true;
pub const MIHOMO_FALLBACK_FILTER_GEOIP_CODE: &str = "CN";
pub const MIHOMO_FALLBACK_FILTER_GEOSITE: [&str; 1] = ["gfw"];
pub const MIHOMO_FALLBACK_FILTER_DOMAIN: [&str; 5] = [
    "+.google.com",
    "+.youtube.com",
    "+.facebook.com",
    "+.twitter.com",
    "+.x.com",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariantKind {
    Service,
    Gui,
    Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigVariant {
    pub id: &'static str,
    pub kind: VariantKind,
    pub script_target: Option<&'static str>,
    pub public_config_file: &'static str,
    pub public_runtime_asset: Option<&'static str>,
    pub upstream_sing_box_asset: Option<&'static str>,
    pub service_backend: &'static str,
    pub tun_profile: &'static str,
    pub route_profile: &'static str,
}

pub const CONFIG_VARIANT_REGISTRY: [ConfigVariant; 7] = [
    ConfigVariant {
        id: "clash-verge",
        kind: VariantKind::Gui,
        script_target: None,
        public_config_file: MIHOMO_PROFILE_FILE,
        public_runtime_asset: None,
        upstream_sing_box_asset: None,
        service_backend: "clash-verge-rev",
        tun_profile: "mihomo",
        route_profile: "mihomo",
    },
    ConfigVariant {
        id: "linux-amd64",
        kind: VariantKind::Service,
        script_target: Some("linux"),
        public_config_file: "linux-amd64.json",
        public_runtime_asset: Some("sing-box-1.13.13-linux-amd64.tar.gz"),
        upstream_sing_box_asset: Some("sing-box-1.13.13-linux-amd64.tar.gz"),
        service_backend: "systemd",
        tun_profile: "linux-service",
        route_profile: "service",
    },
    ConfigVariant {
        id: "linux-arm64",
        kind: VariantKind::Service,
        script_target: Some("linux"),
        public_config_file: "linux-arm64.json",
        public_runtime_asset: Some("sing-box-1.13.13-linux-arm64.tar.gz"),
        upstream_sing_box_asset: Some("sing-box-1.13.13-linux-arm64.tar.gz"),
        service_backend: "systemd",
        tun_profile: "linux-service",
        route_profile: "service",
    },
    ConfigVariant {
        id: "macos-amd64",
        kind: VariantKind::Service,
        script_target: Some("macos"),
        public_config_file: "macos-amd64.json",
        public_runtime_asset: Some("sing-box-1.13.13-macos-amd64.tar.gz"),
        upstream_sing_box_asset: Some("sing-box-1.13.13-darwin-amd64.tar.gz"),
        service_backend: "launchd",
        tun_profile: "macos-service",
        route_profile: "service",
    },
    ConfigVariant {
        id: "macos-arm64",
        kind: VariantKind::Service,
        script_target: Some("macos"),
        public_config_file: "macos-arm64.json",
        public_runtime_asset: Some("sing-box-1.13.13-macos-arm64.tar.gz"),
        upstream_sing_box_asset: Some("sing-box-1.13.13-darwin-arm64.tar.gz"),
        service_backend: "launchd",
        tun_profile: "macos-service",
        route_profile: "service",
    },
    ConfigVariant {
        id: "ios",
        kind: VariantKind::Mobile,
        script_target: None,
        public_config_file: "ios.json",
        public_runtime_asset: None,
        upstream_sing_box_asset: None,
        service_backend: "official app",
        tun_profile: "mobile",
        route_profile: "mobile",
    },
    ConfigVariant {
        id: "android",
        kind: VariantKind::Mobile,
        script_target: None,
        public_config_file: "android.json",
        public_runtime_asset: None,
        upstream_sing_box_asset: None,
        service_backend: "official app",
        tun_profile: "mobile",
        route_profile: "mobile",
    },
];

pub fn sing_box_version_line() -> String {
    format!("sing-box version {SING_BOX_VERSION}")
}

pub fn config_variant(id: &str) -> Option<&'static ConfigVariant> {
    CONFIG_VARIANT_REGISTRY
        .iter()
        .find(|variant| variant.id == id)
}

pub fn service_variants() -> impl Iterator<Item = &'static ConfigVariant> {
    CONFIG_VARIANT_REGISTRY
        .iter()
        .filter(|variant| variant.kind == VariantKind::Service)
}

pub fn managed_server_runtime_variant(os: &str, cpu: &str) -> Option<&'static str> {
    if os.trim() != "Linux" {
        return None;
    }
    match cpu.trim() {
        "x86_64" | "amd64" => Some("linux-amd64"),
        "aarch64" | "arm64" => Some("linux-arm64"),
        _ => None,
    }
}

pub fn is_managed_server_runtime_variant(variant: &str) -> bool {
    MANAGED_SERVER_RUNTIME_VARIANTS.contains(&variant)
}

pub fn release_asset_name(variant: &str) -> Option<&'static str> {
    config_variant(variant).and_then(|entry| entry.public_runtime_asset)
}

pub fn upstream_sing_box_asset_name(variant: &str) -> Option<&'static str> {
    config_variant(variant).and_then(|entry| entry.upstream_sing_box_asset)
}

pub fn upstream_sing_box_url(variant: &str) -> Option<String> {
    upstream_sing_box_asset_name(variant).map(|asset| {
        format!(
            "https://github.com/SagerNet/sing-box/releases/download/{SING_BOX_RELEASE_TAG}/{asset}"
        )
    })
}

pub fn script_extension(target: &str) -> Option<&'static str> {
    match target {
        "linux" | "macos" => Some("sh"),
        _ => None,
    }
}

pub fn is_service_script_target(target: &str) -> bool {
    SERVICE_SCRIPT_TARGETS.contains(&target)
}

pub fn is_config_variant(variant: &str) -> bool {
    CONFIG_VARIANTS.contains(&variant)
}

pub fn all_release_asset_names() -> [&'static str; 6] {
    [
        "sing-box-1.13.13-linux-amd64.tar.gz",
        "sing-box-1.13.13-linux-arm64.tar.gz",
        "sing-box-1.13.13-macos-amd64.tar.gz",
        "sing-box-1.13.13-macos-arm64.tar.gz",
        CN_DOMAIN_PUBLIC_ASSET,
        CN_IPV4_PUBLIC_ASSET,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_registry_matches_v0_0_1_contract() {
        assert_eq!(SERVICE_SCRIPT_TARGETS, ["linux", "macos"]);
        assert_eq!(
            SERVICE_CONFIG_VARIANTS,
            ["linux-amd64", "linux-arm64", "macos-amd64", "macos-arm64"]
        );
        assert_eq!(MOBILE_CONFIG_VARIANTS, ["ios", "android"]);
        assert_eq!(GUI_CONFIG_VARIANTS, ["clash-verge"]);
        assert_eq!(CONFIG_VARIANTS.len(), 7);
        assert_eq!(
            MANAGED_SERVER_RUNTIME_VARIANTS,
            ["linux-amd64", "linux-arm64"]
        );
        assert_eq!(
            CONFIG_VARIANT_REGISTRY.map(|entry| entry.id),
            CONFIG_VARIANTS
        );
        assert_eq!(
            all_release_asset_names(),
            [
                "sing-box-1.13.13-linux-amd64.tar.gz",
                "sing-box-1.13.13-linux-arm64.tar.gz",
                "sing-box-1.13.13-macos-amd64.tar.gz",
                "sing-box-1.13.13-macos-arm64.tar.gz",
                "cn-domain.srs",
                "cn-ipv4.srs",
            ]
        );
        assert_eq!(CN_DOMAIN_RULE_TAG, "cn-domain");
        assert_eq!(CN_IPV4_RULE_TAG, "cn-ipv4");
        assert_eq!(CN_DOMAIN_PUBLIC_ASSET, "cn-domain.srs");
        assert_eq!(CN_IPV4_PUBLIC_ASSET, "cn-ipv4.srs");
        assert!(CN_DOMAIN_UPSTREAM_URL.contains("accelerated-domains.china.conf.srs"));
        assert!(CN_IPV4_UPSTREAM_URL.contains("chnroutes.txt.srs"));
    }

    #[test]
    fn managed_server_runtime_detection_accepts_linux_amd64_and_arm64_only() {
        assert_eq!(
            managed_server_runtime_variant("Linux\n", "x86_64\n"),
            Some("linux-amd64")
        );
        assert_eq!(
            managed_server_runtime_variant("Linux", "amd64"),
            Some("linux-amd64")
        );
        assert_eq!(
            managed_server_runtime_variant("Linux", "aarch64"),
            Some("linux-arm64")
        );
        assert_eq!(
            managed_server_runtime_variant("Linux", "arm64"),
            Some("linux-arm64")
        );
        assert_eq!(managed_server_runtime_variant("Darwin", "arm64"), None);
        assert_eq!(managed_server_runtime_variant("Linux", "riscv64"), None);
    }

    #[test]
    fn network_containment_constants_match_contract() {
        assert_eq!(HEALTH_PROBE_CURL_PROXY_KIND, "socks5-remote-resolve");
        assert_eq!(TUN_IPV4_ADDRESS, "172.19.0.1/30");
        assert_eq!(DNS_STRATEGY, "ipv4_only");
        assert_eq!(DNS_HIJACK_PORT, 53);
        assert_eq!(NETBIRD_DIRECT_CIDR, "100.64.0.0/10");
        assert_eq!(
            NETBIRD_PROCESS_NAMES,
            [
                "netbird.exe",
                "NetBird.exe",
                "netbird",
                "NetBird",
                "netbird-ui",
                "NetBird UI"
            ]
        );
        assert_eq!(
            BUILTIN_DIRECT_IPV4_CIDRS,
            [
                "127.0.0.0/8",
                "169.254.0.0/16",
                "224.0.0.0/4",
                "10.0.0.0/8",
                "172.16.0.0/12",
                "192.168.0.0/16",
            ]
        );
        assert_eq!(CN_DNS_SERVER, "223.5.5.5");
        assert_eq!(CN_DNS_PORT, 443);
        assert_eq!(CN_DNS_TYPE, "https");
        assert_eq!(CN_DNS_PATH, "/dns-query");
        assert_eq!(CN_DNS_TLS_SERVER_NAME, "dns.alidns.com");
        assert_eq!(REMOTE_DNS_SERVER, "1.1.1.1");
        assert_eq!(REMOTE_DNS_PORT, 443);
        assert_eq!(REMOTE_DNS_TYPE, "https");
        assert_eq!(REMOTE_DNS_PATH, "/dns-query");
        assert_eq!(REMOTE_DNS_TLS_SERVER_NAME, "cloudflare-dns.com");
        let no_drop = std::hint::black_box(PUBLIC_IPV6_DENIAL_NO_DROP);
        assert!(no_drop);
        assert_eq!(IPV6_REJECT_NO_DROP, PUBLIC_IPV6_DENIAL_NO_DROP);
    }

    #[test]
    fn macos_public_assets_hide_upstream_darwin_names() {
        let amd64 = config_variant("macos-amd64").unwrap();
        let arm64 = config_variant("macos-arm64").unwrap();
        assert_eq!(
            amd64.public_runtime_asset,
            Some("sing-box-1.13.13-macos-amd64.tar.gz")
        );
        assert_eq!(
            arm64.public_runtime_asset,
            Some("sing-box-1.13.13-macos-arm64.tar.gz")
        );
        assert_eq!(
            amd64.upstream_sing_box_asset,
            Some("sing-box-1.13.13-darwin-amd64.tar.gz")
        );
        assert_eq!(
            arm64.upstream_sing_box_asset,
            Some("sing-box-1.13.13-darwin-arm64.tar.gz")
        );
    }
}
