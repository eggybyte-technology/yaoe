pub fn render_systemd_unit(server: &str) -> String {
    format!(
        r#"[Unit]
Description=YAOE managed Reality egress server {server}
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root
ExecStart=/var/lib/yaoe/bin/sing-box run -c /etc/yaoe/config/{server}.json
Restart=always
RestartSec=5
WorkingDirectory=/var/lib/yaoe
LimitNOFILE=1048576

[Install]
WantedBy=multi-user.target
"#
    )
}

pub fn render_install_sh(server: &str, runtime_variant: &str) -> yaoe_home::YaoeResult<String> {
    let expected_version = yaoe_home::sing_box_version_line();
    let arch_case = match runtime_variant {
        "linux-amd64" => "x86_64|amd64) ;;",
        "linux-arm64" => "aarch64|arm64) ;;",
        _ => {
            return Err(yaoe_home::YaoeError::Internal(format!(
                "unsupported managed server runtime variant: {runtime_variant}"
            )));
        }
    };
    Ok(format!(
        r#"#!/bin/sh
set -eu
SERVER="{server}"
RUNTIME_VARIANT="{runtime_variant}"
EXPECTED_SING_BOX_VERSION="{expected_version}"
SERVICE="yaoe-${{SERVER}}.service"
log() {{ printf 'yaoe [%s] server:%s: %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$SERVER" "$*" >&2; }}

log "validating target host"
test "$(uname -s)" = "Linux"
case "$(uname -m)" in
  {arch_case}
  *) echo "unsupported architecture for $RUNTIME_VARIANT" >&2; exit 1 ;;
esac
test "$(id -u)" -eq 0

if systemctl list-unit-files "$SERVICE" >/dev/null 2>&1; then
  log "stopping existing service"
  systemctl stop "$SERVICE" 2>/dev/null || true
fi

log "installing payload files"
mkdir -p /etc/yaoe/config /var/lib/yaoe/bin /var/log/yaoe
cp payload/bin/sing-box /var/lib/yaoe/bin/sing-box
cp "payload/config/$SERVER.json" "/etc/yaoe/config/$SERVER.json"
chmod 0755 /var/lib/yaoe/bin/sing-box
chmod 0600 "/etc/yaoe/config/$SERVER.json"
touch "/var/log/yaoe/$SERVER.log"
chmod 0600 "/var/log/yaoe/$SERVER.log"

log "checking sing-box version"
version_out="$(/var/lib/yaoe/bin/sing-box version 2>/dev/null)"
first_line="$(printf '%s\n' "$version_out" | sed -n '1p')"
test "$first_line" = "$EXPECTED_SING_BOX_VERSION"
log "validating server config"
/var/lib/yaoe/bin/sing-box check -c "/etc/yaoe/config/$SERVER.json"
log "installing systemd unit"
cp "payload/systemd/$SERVICE" "/etc/systemd/system/$SERVICE"
chmod 0644 "/etc/systemd/system/$SERVICE"
systemctl daemon-reload
log "starting service"
systemctl enable "$SERVICE"
systemctl start "$SERVICE"
test "$(systemctl is-active "$SERVICE")" = "active"
log "service active"
"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_checks_config_before_installing_systemd_unit() {
        let script = render_install_sh("hk", "linux-amd64").unwrap();
        let check_pos = script
            .find("/var/lib/yaoe/bin/sing-box check -c")
            .expect("check command is rendered");
        let unit_copy_pos = script
            .find("cp \"payload/systemd/$SERVICE\"")
            .expect("systemd unit copy is rendered");
        assert!(check_pos < unit_copy_pos);
        assert!(!script.contains("cert"));
    }

    #[test]
    fn installer_checks_expected_package_architecture() {
        let amd64 = render_install_sh("hk", "linux-amd64").unwrap();
        assert!(amd64.contains("RUNTIME_VARIANT=\"linux-amd64\""));
        assert!(amd64.contains("x86_64|amd64) ;;"));
        assert!(!amd64.contains("aarch64|arm64) ;;"));

        let arm64 = render_install_sh("hk", "linux-arm64").unwrap();
        assert!(arm64.contains("RUNTIME_VARIANT=\"linux-arm64\""));
        assert!(arm64.contains("aarch64|arm64) ;;"));
        assert!(!arm64.contains("x86_64|amd64) ;;"));

        assert!(render_install_sh("hk", "macos-arm64").is_err());
    }
}
