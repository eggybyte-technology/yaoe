use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use yaoe_cloudflare::{CloudflareZoneResolver, DomainState, R2Wrangler};
use yaoe_controller::{
    LocalMihomo, LocalSingBox, ProbeRunResult, ProbeSuccess, PublicConfigFetcher,
    RealityKeypairGenerator, RuntimeDeps, cmd_apply, cmd_client, cmd_health, cmd_publish_bootstrap,
    cmd_publish_config, cmd_publish_runtime, cmd_render_config, cmd_status,
};
use yaoe_gitee::{BootstrapFile, GitPublisher, GiteeApi, Release};
use yaoe_home::{YaoeError, YaoeResult};
use yaoe_render::{
    ClientPlatform, ClientRenderInput, HealthProbeRenderInput, ServerRenderInput,
    render_clash_verge_profile, render_client_config, render_health_probe_config,
    render_install_script, render_server_config, render_update_script,
};
use yaoe_rules::{SrsFetcher, SrsValidator};
use yaoe_ssh::{RemoteCommandOutput, SshTransport};
use yaoe_upstream::HttpFetcher;

fn config_key() -> String {
    "A".repeat(128)
}

fn sample_config() -> String {
    format!(
        r#"[ssh]
key = "~/.ssh/id_ed25519"

[cloudflare]
token = "cf_live_token_123456789"
account_id = "account123"
delivery_domain = "cfg.test.net"
r2_bucket = "yaoe-config"

[gitee]
token = "gitee_token_123"
owner = "owner"
repo = "yaoe-delivery"

[credential]
vless_uuid = "550e8400-e29b-41d4-a716-446655440000"
config_key = "{}"
reality_private_key = "sBFVzOLPGBcFMR55fkt7_5xY5SGVD7Vsw8jrLEObh1I"
reality_short_id = "0123456789abcdef"

[reality]
handshake_server = "www.cloudflare.com"

[route]
direct_cidrs = []

[server.hk]
ssh = "root@198.51.100.10"
ip = "198.51.100.10"
port = 28443

[server.jp]
ssh = "root@jp-vps"
ip = "198.51.100.11"
port = 35443
"#,
        config_key()
    )
}

#[test]
fn check_accepts_v0_0_1_config() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    yaoe_home::init_home(&yaoe_home::HomePaths::new(&home)).unwrap();
    fs::write(home.join("yaoe.toml"), sample_config()).unwrap();
    yaoe_controller::cmd_check(Some(&home)).unwrap();
}

#[test]
fn config_rejects_removed_acme_pages_fields() {
    let text = sample_config().replace(
        "r2_bucket = \"yaoe-config\"",
        "r2_bucket = \"yaoe-config\"\nzone = \"test.net\"",
    );
    let err = yaoe_config::parse_and_validate(&text)
        .unwrap_err()
        .to_string();
    assert!(err.contains("cloudflare.zone"));
}

#[test]
fn client_config_renders_reality_urltest_and_direct_srs() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let rendered =
        render_client_config(&ClientRenderInput { config }, ClientPlatform::LinuxAmd64).unwrap();
    let json: Value = serde_json::from_str(&rendered).unwrap();
    assert!(json.get("experimental").is_none());
    assert_eq!(json["dns"]["strategy"], "ipv4_only");
    assert_eq!(json["dns"]["final"], "remote-dns");
    assert_eq!(json["dns"]["servers"][0]["type"], "https");
    assert_eq!(json["dns"]["servers"][0]["tag"], "cn-dns");
    assert_eq!(json["dns"]["servers"][0]["server"], "223.5.5.5");
    assert_eq!(json["dns"]["servers"][0]["server_port"], 443);
    assert_eq!(json["dns"]["servers"][0]["path"], "/dns-query");
    assert!(json["dns"]["servers"][0].get("detour").is_none());
    assert_eq!(
        json["dns"]["servers"][0]["tls"]["server_name"],
        "dns.alidns.com"
    );
    assert_eq!(json["dns"]["servers"][1]["type"], "https");
    assert_eq!(json["dns"]["servers"][1]["tag"], "remote-dns");
    assert_eq!(json["dns"]["servers"][1]["server"], "1.1.1.1");
    assert_eq!(json["dns"]["servers"][1]["server_port"], 443);
    assert_eq!(json["dns"]["servers"][1]["path"], "/dns-query");
    assert_eq!(json["dns"]["servers"][1]["detour"], "proxy");
    assert_eq!(
        json["dns"]["servers"][1]["tls"]["server_name"],
        "cloudflare-dns.com"
    );
    assert_eq!(
        json["dns"]["rules"][0]["domain"],
        serde_json::json!([
            "api.netbird.io",
            "signal.netbird.io",
            "stun.netbird.io",
            "turn.netbird.io"
        ])
    );
    assert_eq!(json["dns"]["rules"][0]["server"], "cn-dns");
    assert_eq!(
        json["dns"]["rules"][1]["domain_suffix"],
        serde_json::json!(["netbird.io", "netbird.cloud", "relay.netbird.io"])
    );
    assert_eq!(
        json["dns"]["rules"][2]["rule_set"],
        serde_json::json!(["cn-domain"])
    );
    assert_eq!(
        json["route"]["rules"][0]["process_name"],
        serde_json::json!(["netbird.exe", "NetBird.exe", "netbird", "NetBird"])
    );
    assert_eq!(json["route"]["rules"][1]["port"], 53);
    assert_eq!(json["route"]["rules"][1]["action"], "hijack-dns");
    assert_eq!(json["inbounds"][0]["auto_redirect"], true);
    assert_eq!(json["inbounds"][0]["mtu"], 1500);
    assert_eq!(json["outbounds"][0]["type"], "urltest");
    assert!(json["outbounds"][0].get("url").is_none());
    assert_eq!(json["outbounds"][1]["tls"]["utls"]["fingerprint"], "chrome");
    assert_eq!(json["route"]["rule_set"][0]["download_detour"], "direct");
    assert_eq!(
        json["inbounds"][0]["address"],
        serde_json::json!(["172.19.0.1/30", "fdfe:dcba:9876::1/126"])
    );
    let cidrs = json["route"]["rules"][5]["ip_cidr"].as_array().unwrap();
    assert_eq!(cidrs[0], "127.0.0.0/8");
    assert!(cidrs.iter().any(|cidr| cidr == "100.64.0.0/10"));
    assert!(cidrs.iter().any(|cidr| cidr == "198.51.100.10/32"));
    assert_eq!(
        json["inbounds"][0]["route_exclude_address"],
        json["route"]["rules"][5]["ip_cidr"]
    );
    assert!(cidrs.iter().any(|cidr| cidr == "::1/128"));
    assert!(cidrs.iter().any(|cidr| cidr == "fe80::/10"));
    assert!(cidrs.iter().any(|cidr| cidr == "fc00::/7"));
    assert!(cidrs.iter().any(|cidr| cidr == "ff00::/8"));
    assert_eq!(json["route"]["rules"][6]["ip_version"], 6);
    assert_eq!(json["route"]["rules"][6]["action"], "reject");
    assert_eq!(json["route"]["rules"][6]["method"], "default");
    assert_eq!(json["route"]["rules"][6]["no_drop"], true);
    assert!(json["route"]["rules"][6].get("outbound").is_none());
    assert_eq!(
        json["route"]["rules"][7]["rule_set"],
        serde_json::json!(["cn-domain", "cn-ipv4"])
    );
    assert!(json["outbounds"][1].get("network").is_none());
    assert_eq!(json["route"]["rule_set"][0]["tag"], "cn-domain");
    assert_eq!(
        json["route"]["rule_set"][0]["url"],
        "https://gitee.com/owner/yaoe-delivery/releases/download/yaoe-v0.0.1-sing-box-1.13.13/cn-domain.srs"
    );
    assert_eq!(json["route"]["rule_set"][1]["tag"], "cn-ipv4");
    assert_eq!(
        json["route"]["rule_set"][1]["url"],
        "https://gitee.com/owner/yaoe-delivery/releases/download/yaoe-v0.0.1-sing-box-1.13.13/cn-ipv4.srs"
    );
}

#[test]
fn non_linux_client_config_omits_auto_redirect() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let rendered =
        render_client_config(&ClientRenderInput { config }, ClientPlatform::MacosArm64).unwrap();
    let json: Value = serde_json::from_str(&rendered).unwrap();
    assert!(json["inbounds"][0].get("auto_redirect").is_none());
    assert_eq!(json["inbounds"][0]["strict_route"], true);
    assert_eq!(json["route"]["auto_detect_interface"], true);
}

#[test]
fn mobile_client_configs_omit_desktop_tun_and_route_fields() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    for platform in [ClientPlatform::Ios, ClientPlatform::Android] {
        let rendered = render_client_config(
            &ClientRenderInput {
                config: config.clone(),
            },
            platform,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&rendered).unwrap();
        assert!(json["inbounds"][0].get("auto_redirect").is_none());
        assert!(json["inbounds"][0].get("strict_route").is_none());
        assert!(json["route"].get("auto_detect_interface").is_none());
        assert_eq!(json["route"]["final"], "proxy");
        assert_eq!(json["outbounds"][0]["tag"], "proxy");
        assert_eq!(
            json["inbounds"][0]["address"],
            serde_json::json!(["172.19.0.1/30", "fdfe:dcba:9876::1/126"])
        );
        assert_eq!(
            json["inbounds"][0]["route_exclude_address"],
            json["route"]["rules"][4]["ip_cidr"]
        );
        assert_eq!(json["route"]["rules"][5]["ip_version"], 6);
        assert_eq!(json["route"]["rules"][5]["no_drop"], true);
    }
}

#[test]
fn all_platform_configs_pass_sing_box_check_from_path() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    for platform in [
        ClientPlatform::LinuxAmd64,
        ClientPlatform::LinuxArm64,
        ClientPlatform::MacosAmd64,
        ClientPlatform::MacosArm64,
        ClientPlatform::Ios,
        ClientPlatform::Android,
    ] {
        let rendered = render_client_config(
            &ClientRenderInput {
                config: config.clone(),
            },
            platform,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(json["dns"]["strategy"], "ipv4_only");
        let first_dns_rule = if platform.is_service() { 1 } else { 0 };
        assert_eq!(
            json["route"]["rules"][first_dns_rule]["action"],
            "hijack-dns"
        );
        assert!(!rendered.contains(r#""type": "udp""#));
        assert!(!rendered.contains(r#""type": "system""#));
        assert!(!rendered.contains(r#""type": "dhcp""#));
        assert!(!rendered.contains("dns64"));
        assert!(!rendered.contains("nat64"));
        assert!(!rendered.contains("prefer_ipv6"));
        assert!(!rendered.contains("ipv6_only"));
        let path = dir.path().join(format!("{}.json", platform.as_str()));
        fs::write(&path, rendered).unwrap();
        let output = Command::new("sing-box")
            .arg("check")
            .arg("-c")
            .arg(&path)
            .output()
            .expect("run sing-box check");
        assert!(
            output.status.success(),
            "{} failed sing-box check: {}",
            platform.as_str(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn clash_verge_profile_renders_mihomo_yaml_contract() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let rendered = render_clash_verge_profile(&ClientRenderInput { config }).unwrap();

    assert!(rendered.starts_with("mixed-port: 7890\nallow-lan: false\nmode: rule\n"));
    assert!(rendered.contains("geox-url:\n  geoip: https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat\n"));
    assert!(rendered.contains("dns:\n  enable: true\n  ipv6: false\n  enhanced-mode: fake-ip\n"));
    assert!(rendered.contains("  fake-ip-filter:\n    - \"netbird.io\"\n"));
    assert!(rendered.contains("  direct-nameserver-follow-policy: true\n"));
    assert!(rendered.contains("    - https://1.1.1.1/dns-query#PROXY\n"));
    assert!(rendered.contains("    geosite:\n      - gfw\n"));
    assert!(rendered.contains("tun:\n  enable: true\n  stack: mixed\n"));
    assert!(rendered.contains("  route-exclude-address:\n    - 127.0.0.0/8\n"));
    assert!(rendered.contains("  - name: egress-hk\n    type: vless\n"));
    assert!(rendered.contains("    reality-opts:\n      public-key:"));
    assert!(rendered.contains("      support-x25519mlkem768: false\n"));
    assert!(rendered.contains("proxy-groups:\n  - name: PROXY\n    type: url-test\n"));
    assert!(rendered.contains("  - PROCESS-NAME,netbird.exe,DIRECT\n"));
    assert!(rendered.contains("  - IP-CIDR,100.64.0.0/10,DIRECT,no-resolve\n"));
    assert!(rendered.contains("  - DOMAIN,api.netbird.io,DIRECT\n"));
    assert!(rendered.contains("  - DOMAIN-SUFFIX,netbird.io,DIRECT\n"));
    assert!(rendered.contains("  - GEOSITE,cn,DIRECT\n  - GEOIP,CN,DIRECT\n  - MATCH,PROXY\n"));
    assert!(!rendered.contains("windows"));
    assert!(!rendered.contains("::1/128"));
}

#[test]
fn clash_verge_profile_passes_mihomo_check_from_path() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let rendered = render_clash_verge_profile(&ClientRenderInput { config }).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clash-verge.yaml");
    fs::write(&path, rendered).unwrap();

    let output = Command::new("mihomo")
        .arg("-t")
        .arg("-f")
        .arg(&path)
        .output()
        .expect("run mihomo check");
    assert!(
        output.status.success(),
        "mihomo check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn client_derives_entrypoints_without_full_placeholder_validation() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir_all(&home).unwrap();
    let partial = format!(
        r#"[cloudflare]
delivery_domain = "cfg.test.net"

[gitee]
owner = "owner"
repo = "repo"

[credential]
config_key = "{}"
"#,
        config_key()
    );
    fs::write(home.join("yaoe.toml"), partial).unwrap();
    cmd_client(Some(&home)).unwrap();
}

#[test]
fn server_config_is_reality_without_certificate_fields() {
    let rendered = render_server_config(&ServerRenderInput {
        server_name: "hk".into(),
        endpoint_ip: "198.51.100.10".into(),
        port: 28443,
        vless_uuid: "550e8400-e29b-41d4-a716-446655440000".into(),
        reality_private_key: "sBFVzOLPGBcFMR55fkt7_5xY5SGVD7Vsw8jrLEObh1I".into(),
        reality_short_id: "0123456789abcdef".into(),
        handshake_server: "www.cloudflare.com".into(),
        handshake_port: 443,
    })
    .unwrap();
    let json: Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(
        json["inbounds"][0]["tls"]["server_name"],
        "www.cloudflare.com"
    );
    assert_eq!(
        json["inbounds"][0]["tls"]["reality"]["handshake"]["server"],
        "www.cloudflare.com"
    );
    assert!(rendered.contains("\"reality\""));
    assert_eq!(json["inbounds"][0]["listen"], "0.0.0.0");
    assert!(!rendered.contains("certificate_path"));
    assert!(!rendered.contains("acme"));
}

#[test]
fn server_and_health_probe_configs_pass_sing_box_check_from_path() {
    let private_key = "sBFVzOLPGBcFMR55fkt7_5xY5SGVD7Vsw8jrLEObh1I";
    let public_key = yaoe_config::derive_reality_public_key(private_key).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let server_config = render_server_config(&ServerRenderInput {
        server_name: "hk".into(),
        endpoint_ip: "198.51.100.10".into(),
        port: 28443,
        vless_uuid: "550e8400-e29b-41d4-a716-446655440000".into(),
        reality_private_key: private_key.into(),
        reality_short_id: "0123456789abcdef".into(),
        handshake_server: "www.cloudflare.com".into(),
        handshake_port: 443,
    })
    .unwrap();
    let probe_config = render_health_probe_config(&HealthProbeRenderInput {
        endpoint_ip: "198.51.100.10".into(),
        port: 28443,
        probe_port: 2080,
        vless_uuid: "550e8400-e29b-41d4-a716-446655440000".into(),
        reality_public_key: public_key,
        reality_short_id: "0123456789abcdef".into(),
        handshake_server: "www.cloudflare.com".into(),
    })
    .unwrap();
    for (name, rendered) in [("server.json", server_config), ("probe.json", probe_config)] {
        let path = dir.path().join(name);
        fs::write(&path, rendered).unwrap();
        let output = Command::new("sing-box")
            .arg("check")
            .arg("-c")
            .arg(&path)
            .output()
            .expect("run sing-box check");
        assert!(
            output.status.success(),
            "{name} failed sing-box check: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn scripts_construct_config_url_from_env_key() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let install = render_install_script(&config, "linux").unwrap();
    let update = render_update_script(&config, "macos").unwrap();
    assert!(install.contains("variant=\"linux-$arch\""));
    assert!(install.contains("/config/$YAOE_CONFIG_KEY/$variant.json"));
    assert!(install.contains("aarch64|arm64) arch=\"arm64\""));
    assert!(update.contains("variant=\"macos-$arch\""));
    assert!(update.contains("/config/$YAOE_CONFIG_KEY/$variant.json"));
    assert!(update.contains("arm64) arch=\"arm64\""));
    assert!(!install.contains(&config.credential.config_key));
    for rendered in [
        install,
        update,
        render_install_script(&config, "macos").unwrap(),
        render_update_script(&config, "macos").unwrap(),
    ] {
        assert!(!rendered.contains("darwin-"));
        assert!(!rendered.contains("linux-amd64.sh"));
        assert!(!rendered.contains("macos-amd64.sh"));
    }
}

#[test]
fn service_scripts_probe_runtime_startup_and_log_launchd_output() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let linux_install = render_install_script(&config, "linux").unwrap();
    let linux_update = render_update_script(&config, "linux").unwrap();
    let macos_install = render_install_script(&config, "macos").unwrap();
    let macos_update = render_update_script(&config, "macos").unwrap();

    for rendered in [&linux_install, &linux_update, &macos_install, &macos_update] {
        assert!(rendered.contains("log() { printf 'yaoe: %s\\n' \"$*\" >&2; }"));
        assert!(rendered.contains("validated root privileges and config key shape"));
        assert!(rendered.contains("checking sing-box version"));
        assert!(rendered.contains("pending sing-box config OK"));
        assert!(rendered.contains("waiting for immediate sing-box runtime failures"));
        assert!(rendered.contains("smoke_probe()"));
        assert!(rendered.contains("https://www.google.com/generate_204|204"));
        assert!(rendered.contains("https://www.gstatic.com/generate_204"));
        assert!(rendered.contains("https://github.com|200"));
        assert!(rendered.contains("https://api.github.com/rate_limit|200"));
        assert!(rendered.contains("-w '%{http_code}'"));
        assert!(rendered.contains("service smoke probe OK: url=$url http=$status"));
        assert!(rendered.contains("curl_exit=$curl_exit"));
        assert!(
            rendered.contains("WARNING: service smoke probe did not reach public test endpoints")
        );
        assert!(rendered.contains(
            "if smoke_probe; then smoke_result=\"ok\"; else smoke_result=\"warning\"; fi"
        ));
        assert!(!rendered.contains("fail \"service smoke probe failed"));
        assert!(!rendered.contains("downloading $1 from $2"));
    }

    assert!(
        linux_install
            .contains("service_state=\"$(systemctl is-active yaoe-sing-box.service || true)\"")
    );
    assert!(linux_install.contains("starting YAOE sing-box linux install"));
    assert!(linux_update.contains("starting YAOE sing-box linux update"));
    assert!(linux_install.contains("systemd state: yaoe-sing-box.service=$service_state"));
    assert!(linux_install.contains(
        "YAOE sing-box linux install completed: service=active smoke_probe=$smoke_result"
    ));
    assert!(macos_install.contains("starting YAOE sing-box macos install"));
    assert!(macos_update.contains("starting YAOE sing-box macos update"));
    assert!(macos_install.contains("<key>StandardOutPath</key>"));
    assert!(macos_install.contains("<string>/Library/Logs/YAOE/sing-box.out.log</string>"));
    assert!(macos_install.contains("<key>StandardErrorPath</key>"));
    assert!(macos_install.contains("<string>/Library/Logs/YAOE/sing-box.err.log</string>"));
    assert!(macos_install.contains("launchd state: io.yaoe.sing-box=${launchd_state:-unknown}"));
    assert!(macos_install.contains(
        "YAOE sing-box macos install completed: service=running smoke_probe=$smoke_result"
    ));
}

#[test]
fn publish_bootstrap_uses_git_publish_path() {
    let (_dir, home) = home_with_config();
    let calls = Calls::default();
    let deps = fake_deps(calls.clone());

    cmd_publish_bootstrap(Some(&home), &deps).unwrap();

    let calls = calls.snapshot();
    assert!(calls.iter().any(|call| call == "gitee.ensure_repository"));
    assert!(calls.iter().any(|call| call == "git.baseline:4"));
    assert!(calls.iter().any(|call| call == "git.publish:4"));
    assert!(calls.iter().any(|call| {
        call == "git.publish.paths:install/linux.sh,install/macos.sh,update/linux.sh,update/macos.sh"
    }));
}

#[test]
fn publish_runtime_only_ensures_bootstrap_baseline_before_release_assets() {
    let (_dir, home) = home_with_config();
    let calls = Calls::default();
    let deps = fake_deps(calls.clone());

    cmd_publish_runtime(Some(&home), &deps).unwrap();

    let calls = calls.snapshot();
    let repo = calls
        .iter()
        .position(|call| call == "gitee.ensure_repository")
        .expect("runtime ensures repository");
    let baseline = calls
        .iter()
        .position(|call| call == "git.baseline:4")
        .expect("runtime ensures bootstrap baseline");
    let release = calls
        .iter()
        .position(|call| call == "gitee.ensure_release")
        .expect("runtime ensures release");
    assert!(repo < baseline);
    assert!(baseline < release);
    assert!(!calls.iter().any(|call| call.starts_with("git.publish")));
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("gitee.upload_asset:"))
            .count(),
        6
    );
    assert!(
        calls
            .iter()
            .any(|call| call.contains("srs.validate:cn-domain:"))
    );
    assert!(
        calls
            .iter()
            .any(|call| call.contains("srs.validate:cn-ipv4:"))
    );
}

#[test]
fn publish_config_puts_seven_config_objects_and_checks_public_results() {
    let (_dir, home) = home_with_config();
    let calls = Calls::default();
    let deps = fake_deps(calls.clone());

    cmd_publish_config(Some(&home), &deps).unwrap();

    let calls = calls.snapshot();
    let put_objects: Vec<_> = calls
        .iter()
        .filter_map(|call| call.strip_prefix("r2.put:"))
        .map(str::to_string)
        .collect();
    let expected_put_objects: Vec<_> = yaoe_home::CONFIG_VARIANTS
        .iter()
        .map(|variant| {
            let file = yaoe_home::config_variant(variant)
                .unwrap()
                .public_config_file;
            let content_type = if *variant == "clash-verge" {
                yaoe_home::R2_YAML_CONTENT_TYPE
            } else {
                yaoe_home::R2_JSON_CONTENT_TYPE
            };
            format!("config/{}/{file}:{content_type}", config_key())
        })
        .collect();
    assert_eq!(put_objects, expected_put_objects);
    assert!(put_objects.iter().all(|key| key.starts_with("config/")));
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("public.fetch:https://cfg.test.net/config/"))
            .count(),
        7
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("sing-box.check:"))
            .count(),
        12
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("mihomo.check:"))
            .count(),
        2
    );
    let rendered_entries = fs::read_dir(home.join("work/delivery/rendered-config"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        !rendered_entries
            .iter()
            .any(|name| name.starts_with(".public-")),
        "public fetch validation temp files should not remain: {rendered_entries:?}"
    );
}

#[test]
fn render_config_checks_seven_local_configs_without_network_calls() {
    let (_dir, home) = home_with_config();
    let rendered_config_dir = home.join("work/delivery/rendered-config");
    fs::create_dir_all(&rendered_config_dir).unwrap();
    fs::write(rendered_config_dir.join("windows-amd64.json"), "{}").unwrap();
    fs::write(rendered_config_dir.join(".public-clash-verge.yaml"), "{}").unwrap();
    let calls = Calls::default();
    let deps = fake_deps(calls.clone());

    cmd_render_config(Some(&home), &deps).unwrap();

    let calls = calls.snapshot();
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.as_str() == "sing-box.version")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.as_str() == "mihomo.version")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("sing-box.check:"))
            .count(),
        6
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.starts_with("mihomo.check:"))
            .count(),
        1
    );
    assert!(!calls.iter().any(|call| {
        call.starts_with("cloudflare.")
            || call.starts_with("r2.")
            || call.starts_with("gitee.")
            || call.starts_with("git.")
            || call.starts_with("ssh:")
            || call.starts_with("public.fetch:")
    }));
    let mut expected_files = Vec::new();
    for variant in yaoe_home::CONFIG_VARIANTS {
        let file = yaoe_home::config_variant(variant)
            .unwrap()
            .public_config_file;
        expected_files.push(file.to_string());
        assert!(rendered_config_dir.join(file).is_file());
    }
    expected_files.sort();
    let mut rendered_files = fs::read_dir(&rendered_config_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    rendered_files.sort();
    assert_eq!(rendered_files, expected_files);
}

#[test]
fn update_scripts_do_not_replace_first_install_artifacts() {
    let config = yaoe_config::parse_and_validate(&sample_config()).unwrap();
    let linux = render_update_script(&config, "linux").unwrap();
    let macos = render_update_script(&config, "macos").unwrap();

    assert!(!linux.contains("sing-box.tar.gz"));
    assert!(!linux.contains("systemctl daemon-reload"));
    assert!(!macos.contains("sing-box.tar.gz"));
    assert!(!macos.contains("LaunchDaemons/io.yaoe.sing-box.plist <<"));
}

#[test]
fn status_runs_full_remote_validation_sequence() {
    let (_dir, home) = home_with_config();
    let calls = Calls::default();
    let deps = fake_deps(calls.clone());

    cmd_status(Some(&home), Some("hk"), &deps).unwrap();

    let calls = calls.snapshot();
    assert!(
        calls
            .iter()
            .any(|call| call == "ssh:systemctl is-active yaoe-hk.service")
    );
    assert!(
        calls
            .iter()
            .any(|call| call == "ssh:systemctl is-enabled yaoe-hk.service")
    );
    assert!(
        calls
            .iter()
            .any(|call| call == "ssh:systemctl show yaoe-hk.service --property=MainPID --value")
    );
    assert!(
        calls
            .iter()
            .any(|call| call == "ssh:/var/lib/yaoe/bin/sing-box version")
    );
    assert!(
        calls
            .iter()
            .any(|call| call == "ssh:/var/lib/yaoe/bin/sing-box check -c /etc/yaoe/config/hk.json")
    );
    assert!(
        calls
            .iter()
            .any(|call| call == "ssh:ss -H -ltn sport = :28443")
    );
}

#[test]
fn apply_detects_remote_arm64_and_packages_matching_runtime() {
    let (_dir, home) = home_with_config();
    write_cached_server_runtime(&home, "linux-arm64");
    let calls = Calls::default();
    let deps = fake_deps_with_platform(calls.clone(), "Linux", "aarch64");

    cmd_apply(Some(&home), Some("hk"), &deps).unwrap();

    let calls = calls.snapshot();
    let os = calls
        .iter()
        .position(|call| call == "ssh:uname -s")
        .expect("apply detects remote OS");
    let arch = calls
        .iter()
        .position(|call| call == "ssh:uname -m")
        .expect("apply detects remote CPU");
    let upload = calls
        .iter()
        .position(|call| call.starts_with("ssh.upload:"))
        .expect("apply uploads package");
    assert!(os < upload);
    assert!(arch < upload);

    let install =
        fs::read_to_string(home.join("work/packages/hk/yaoe-server-package/install.sh")).unwrap();
    assert!(install.contains("RUNTIME_VARIANT=\"linux-arm64\""));
    assert!(install.contains("aarch64|arm64) ;;"));
}

#[test]
fn apply_rejects_unsupported_remote_server_architecture_before_packaging() {
    let (_dir, home) = home_with_config();
    let calls = Calls::default();
    let deps = fake_deps_with_platform(calls.clone(), "Linux", "riscv64");

    let err = cmd_apply(Some(&home), Some("hk"), &deps)
        .unwrap_err()
        .to_string();

    assert!(err.contains("unsupported managed server platform"));
    assert!(!home.join("work/packages/hk/yaoe-server-package").exists());
}

#[test]
fn health_renders_mixed_probe_and_runs_local_probe() {
    let (_dir, home) = home_with_config();
    let calls = Calls::default();
    let deps = fake_deps(calls.clone());

    cmd_health(Some(&home), Some("hk"), &deps).unwrap();

    let probe = home.join("work/health/hk/probe.json");
    let json: Value = serde_json::from_str(&fs::read_to_string(probe).unwrap()).unwrap();
    assert_eq!(json["inbounds"][0]["type"], "mixed");
    assert_eq!(json["inbounds"][0]["listen"], "127.0.0.1");
    assert_eq!(json["outbounds"][0]["type"], "vless");
    assert_eq!(
        json["outbounds"][0]["tls"]["server_name"],
        "www.cloudflare.com"
    );
    assert_eq!(json["route"]["final"], "probe");
    assert!(json.get("dns").is_none());
    assert!(!json.to_string().contains("auto_route"));

    let calls = calls.snapshot();
    assert!(calls.iter().any(|call| call == "sing-box.version"));
    assert!(calls.iter().any(|call| call.starts_with("sing-box.probe:")));
}

#[test]
fn readme_documents_os_level_client_blocks_and_ipv6_containment() {
    let readme = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("repo root")
            .join("README.md"),
    )
    .unwrap();
    for forbidden in [
        "linux-amd64 install",
        "linux-arm64 install",
        "macos-amd64 install",
        "macos-arm64 install",
        "darwin-",
    ] {
        assert!(!readme.contains(forbidden), "README contains {forbidden}");
    }
    assert!(readme.contains("linux sing-box"));
    assert!(readme.contains("Clash Verge Rev"));
    assert!(readme.contains("macos sing-box"));
    assert!(readme.contains("Generated configs implement IPv4 egress semantics"));
    assert!(readme.contains("NetBird overlay traffic"));
    assert!(readme.contains("NetBird control/STUN/TURN/relay traffic"));
    assert!(readme.contains("configured direct CIDRs"));
    assert!(readme.contains("managed-server endpoint `/32` addresses"));
    assert!(readme.contains("CN allowlist traffic"));
    assert!(readme.contains("remaining public IPv4 traffic uses proxy aggregation"));
    assert!(readme.contains("Clash Verge Rev users edit no rule files"));
    assert!(readme.contains(
        "cargo nextest run -p yaoe-controller -P acceptance --run-ignored=only acceptance_delivery"
    ));
}

#[test]
fn readme_first_time_flow_and_rotation_commands_match_contract() {
    let readme = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("repo root")
            .join("README.md"),
    )
    .unwrap();
    assert_ordered(
        &readme,
        &[
            "direnv allow",
            "cargo install --path crates/yaoe-cli --locked --force",
            "yaoe init",
            "Edit `.yaoe/yaoe.toml`",
            "yaoe check",
            "yaoe render config",
            "yaoe publish delivery",
            "yaoe apply",
            "yaoe status",
            "yaoe health",
            "yaoe client",
        ],
    );
    assert_ordered(
        &readme,
        &[
            "yaoe publish bootstrap",
            "yaoe publish runtime",
            "yaoe publish config",
        ],
    );
    for flow in [
        "yaoe rotate config-key\nyaoe publish config",
        "yaoe rotate vless-uuid\nyaoe apply\nyaoe publish config",
        "yaoe rotate reality-keypair\nyaoe apply\nyaoe publish config",
    ] {
        assert!(
            readme.contains(flow),
            "README missing rotation flow: {flow}"
        );
    }
}

fn assert_ordered(text: &str, needles: &[&str]) {
    let mut offset = 0;
    for needle in needles {
        let found = text[offset..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered README text: {needle}"));
        offset += found + needle.len();
    }
}

fn home_with_config() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("yaoe.toml"), sample_config()).unwrap();
    (dir, home)
}

#[derive(Clone, Default)]
struct Calls(Arc<Mutex<Vec<String>>>);

impl Calls {
    fn push(&self, call: impl Into<String>) {
        self.0.lock().unwrap().push(call.into());
    }

    fn snapshot(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

fn fake_deps(calls: Calls) -> RuntimeDeps {
    fake_deps_with_platform(calls, "Linux", "x86_64")
}

fn fake_deps_with_platform(calls: Calls, os: &'static str, arch: &'static str) -> RuntimeDeps {
    RuntimeDeps {
        cloudflare: Box::new(FakeCloudflare {
            calls: calls.clone(),
        }),
        r2: Box::new(FakeR2 {
            calls: calls.clone(),
        }),
        gitee: Box::new(FakeGitee {
            calls: calls.clone(),
        }),
        git: Box::new(FakeGit {
            calls: calls.clone(),
        }),
        upstream_fetcher: Box::new(FakeFetcher),
        srs_fetcher: Box::new(FakeSrsFetcher),
        srs_validator: Box::new(FakeSrsValidator {
            calls: calls.clone(),
        }),
        ssh: Box::new(FakeSsh {
            calls: calls.clone(),
            os,
            arch,
        }),
        local_sing_box: Box::new(FakeSingBox {
            calls: calls.clone(),
        }),
        local_mihomo: Box::new(FakeMihomo {
            calls: calls.clone(),
        }),
        reality_keypair: Box::new(FakeRealityKeypair),
        public_config_fetcher: Box::new(FakePublicConfigFetcher {
            calls: calls.clone(),
        }),
    }
}

fn write_cached_server_runtime(home: &Path, variant: &str) {
    let path = home
        .join("cache/server-runtime/sing-box/1.13.13")
        .join(variant)
        .join("sing-box");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nprintf '{}\\n'\n",
            yaoe_home::sing_box_version_line()
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

struct FakeCloudflare {
    calls: Calls,
}

impl CloudflareZoneResolver for FakeCloudflare {
    fn resolve_zone_id(&self, delivery_domain: &str) -> YaoeResult<String> {
        self.calls
            .push(format!("cloudflare.resolve:{delivery_domain}"));
        Ok("zone-id".into())
    }
}

struct FakeR2 {
    calls: Calls,
}

impl R2Wrangler for FakeR2 {
    fn bucket_exists(&self, _: &str, _: &str, bucket: &str) -> YaoeResult<bool> {
        self.calls.push(format!("r2.bucket_exists:{bucket}"));
        Ok(true)
    }

    fn create_bucket(&self, _: &str, _: &str, bucket: &str) -> YaoeResult<()> {
        self.calls.push(format!("r2.create_bucket:{bucket}"));
        Ok(())
    }

    fn domain_state(
        &self,
        _: &str,
        _: &str,
        _: &str,
        domain: &str,
    ) -> YaoeResult<Option<DomainState>> {
        self.calls.push(format!("r2.domain_state:{domain}"));
        Ok(Some(DomainState {
            min_tls: Some("1.2".into()),
        }))
    }

    fn add_domain(&self, _: &str, _: &str, _: &str, domain: &str, _: &str) -> YaoeResult<()> {
        self.calls.push(format!("r2.add_domain:{domain}"));
        Ok(())
    }

    fn update_domain_tls(&self, _: &str, _: &str, _: &str, domain: &str) -> YaoeResult<()> {
        self.calls.push(format!("r2.update_domain_tls:{domain}"));
        Ok(())
    }

    fn put_object(
        &self,
        _: &str,
        _: &str,
        _: &str,
        object_key: &str,
        _: &Path,
        content_type: &str,
    ) -> YaoeResult<()> {
        self.calls
            .push(format!("r2.put:{object_key}:{content_type}"));
        Ok(())
    }
}

struct FakeGitee {
    calls: Calls,
}

impl GiteeApi for FakeGitee {
    fn authenticated_login(&self) -> YaoeResult<String> {
        self.calls.push("gitee.authenticated_login");
        Ok("owner".into())
    }

    fn ensure_repository(&self, _: &str, _: &str) -> YaoeResult<()> {
        self.calls.push("gitee.ensure_repository");
        Ok(())
    }

    fn ensure_release(&self, _: &str, _: &str) -> YaoeResult<Release> {
        self.calls.push("gitee.ensure_release");
        Ok(Release { id: 1 })
    }

    fn release_asset_names(&self, _: &str, _: &str, _: u64) -> YaoeResult<Vec<String>> {
        self.calls.push("gitee.release_asset_names");
        Ok(Vec::new())
    }

    fn upload_release_asset(&self, _: &str, _: &str, _: u64, file: &Path) -> YaoeResult<()> {
        let name = file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        self.calls.push(format!("gitee.upload_asset:{name}"));
        Ok(())
    }
}

struct FakeGit {
    calls: Calls,
}

impl GitPublisher for FakeGit {
    fn ensure_branch_baseline(
        &self,
        _: &yaoe_home::HomePaths,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        files: &[BootstrapFile],
    ) -> YaoeResult<()> {
        self.calls.push(format!("git.baseline:{}", files.len()));
        self.calls.push(format!(
            "git.baseline.paths:{}",
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ));
        Ok(())
    }

    fn publish_bootstrap_files(
        &self,
        _: &yaoe_home::HomePaths,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        files: &[BootstrapFile],
    ) -> YaoeResult<()> {
        self.calls.push(format!("git.publish:{}", files.len()));
        self.calls.push(format!(
            "git.publish.paths:{}",
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ));
        Ok(())
    }
}

struct FakeFetcher;

impl HttpFetcher for FakeFetcher {
    fn fetch(&self, _url: &str) -> YaoeResult<Vec<u8>> {
        Ok(b"artifact".to_vec())
    }
}

struct FakeSrsFetcher;

impl SrsFetcher for FakeSrsFetcher {
    fn fetch_srs(&self, _url: &str) -> YaoeResult<Vec<u8>> {
        Ok(b"srs".to_vec())
    }
}

struct FakeSrsValidator {
    calls: Calls,
}

impl SrsValidator for FakeSrsValidator {
    fn validate_binary_rule_set(&self, path: &Path, tag: &str) -> YaoeResult<()> {
        self.calls
            .push(format!("srs.validate:{tag}:{}", path.display()));
        Ok(())
    }
}

struct FakeSsh {
    calls: Calls,
    os: &'static str,
    arch: &'static str,
}

impl SshTransport for FakeSsh {
    fn upload(&self, _: &str, _: &str, remote_path: &str, _: &str) -> YaoeResult<()> {
        self.calls.push(format!("ssh.upload:{remote_path}"));
        Ok(())
    }

    fn run_as_root_raw(&self, _: &str, command: &str, _: &str) -> YaoeResult<RemoteCommandOutput> {
        self.calls.push(format!("ssh:{command}"));
        let stdout = if command == "uname -s" {
            format!("{}\n", self.os)
        } else if command == "uname -m" {
            format!("{}\n", self.arch)
        } else if command.contains("systemctl is-active") {
            "active\n".to_string()
        } else if command.contains("systemctl is-enabled") {
            "enabled\n".to_string()
        } else if command.contains("MainPID") {
            "42\n".to_string()
        } else if command == "/var/lib/yaoe/bin/sing-box version" {
            format!("{}\n", yaoe_home::sing_box_version_line())
        } else if command.contains("sing-box check -c") {
            String::new()
        } else if command.contains("ss -H -ltn sport = :28443") {
            "LISTEN 0 4096 0.0.0.0:28443 0.0.0.0:*\n".to_string()
        } else if command.contains("ss -H -ltn sport = :35443") {
            "LISTEN 0 4096 0.0.0.0:35443 0.0.0.0:*\n".to_string()
        } else {
            String::new()
        };
        Ok(RemoteCommandOutput {
            status: 0,
            stdout,
            stderr: String::new(),
        })
    }

    fn read_file(&self, _: &str, _: &str, _: &str) -> YaoeResult<String> {
        Ok(String::new())
    }
}

struct FakeSingBox {
    calls: Calls,
}

impl LocalSingBox for FakeSingBox {
    fn require_version(&self) -> YaoeResult<()> {
        self.calls.push("sing-box.version");
        Ok(())
    }

    fn check_config(&self, path: &Path) -> YaoeResult<()> {
        self.calls
            .push(format!("sing-box.check:{}", path.display()));
        Ok(())
    }

    fn run_health_probe(&self, path: &Path, probe_port: u16, _: &str) -> ProbeRunResult {
        self.calls
            .push(format!("sing-box.probe:{}:{probe_port}", path.display()));
        Ok(ProbeSuccess {
            status: 204,
            elapsed_ms: 7,
            pid: 123,
        })
    }
}

struct FakeMihomo {
    calls: Calls,
}

impl LocalMihomo for FakeMihomo {
    fn require_version(&self) -> YaoeResult<()> {
        self.calls.push("mihomo.version");
        Ok(())
    }

    fn check_config(&self, path: &Path) -> YaoeResult<()> {
        self.calls.push(format!("mihomo.check:{}", path.display()));
        Ok(())
    }
}

struct FakeRealityKeypair;

impl RealityKeypairGenerator for FakeRealityKeypair {
    fn generate(&self) -> YaoeResult<(String, String)> {
        Err(YaoeError::Internal("not used".into()))
    }
}

struct FakePublicConfigFetcher {
    calls: Calls,
}

impl PublicConfigFetcher for FakePublicConfigFetcher {
    fn fetch_ok(&self, url: &str) -> YaoeResult<Option<Vec<u8>>> {
        self.calls.push(format!("public.fetch:{url}"));
        Ok(Some(b"{}".to_vec()))
    }
}
