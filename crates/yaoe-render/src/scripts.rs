use yaoe_config::Config;
use yaoe_home::{
    GITEE_BOOTSTRAP_BRANCH, GITEE_RELEASE_TAG, SING_BOX_VERSION, YaoeError, YaoeResult,
    script_extension,
};

pub fn render_install_script(config: &Config, target: &str) -> YaoeResult<String> {
    match target {
        "linux" => Ok(render_linux_script(config, true)),
        "macos" => Ok(render_macos_script(config, true)),
        _ => Err(YaoeError::Internal(format!(
            "unsupported service script target: {target}"
        ))),
    }
}

pub fn render_update_script(config: &Config, target: &str) -> YaoeResult<String> {
    match target {
        "linux" => Ok(render_linux_script(config, false)),
        "macos" => Ok(render_macos_script(config, false)),
        _ => Err(YaoeError::Internal(format!(
            "unsupported service script target: {target}"
        ))),
    }
}

pub fn raw_script_url(config: &Config, kind: &str, target: &str) -> YaoeResult<String> {
    let ext = script_extension(target).ok_or_else(|| {
        YaoeError::Internal(format!("unsupported service script target: {target}"))
    })?;
    Ok(format!(
        "https://gitee.com/{}/{}/raw/{}/{}/{}.{}",
        config.gitee.owner, config.gitee.repo, GITEE_BOOTSTRAP_BRANCH, kind, target, ext
    ))
}

fn config_base(config: &Config) -> String {
    format!("https://{}", config.cloudflare.delivery_domain)
}

fn release_base(config: &Config) -> String {
    format!(
        "https://gitee.com/{}/{}/releases/download/{}",
        config.gitee.owner, config.gitee.repo, GITEE_RELEASE_TAG
    )
}

fn render_linux_script(config: &Config, install: bool) -> String {
    let config_base = config_base(config);
    let release_base = release_base(config);
    let install_block = if install {
        format!(
            r#"
install -d -m 0755 /usr/local/libexec/yaoe
install -d -m 0755 /etc/yaoe-sing-box
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
archive_url="{release_base}/sing-box-{SING_BOX_VERSION}-linux-$arch.tar.gz"
download "sing-box archive" "$archive_url" "$tmp/sing-box.tar.gz"
tar -xzf "$tmp/sing-box.tar.gz" -C "$tmp"
bin="$(find "$tmp" -type f -name sing-box | head -n 1)"
[ -n "$bin" ] || fail "sing-box not found in archive"
install -m 0755 "$bin" /usr/local/libexec/yaoe/sing-box
"#
        )
    } else {
        r#"
[ -x /usr/local/libexec/yaoe/sing-box ] || fail "missing sing-box executable"
[ -f /etc/systemd/system/yaoe-sing-box.service ] || fail "missing service definition"
install -d -m 0755 /etc/yaoe-sing-box
"#
        .to_string()
    };
    let service_block = if install {
        r#"
cat > /etc/systemd/system/yaoe-sing-box.service <<'UNIT'
[Unit]
Description=YAOE sing-box client
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root
ExecStart=/usr/local/libexec/yaoe/sing-box run -c /etc/yaoe-sing-box/config.json
Restart=always
RestartSec=5
WorkingDirectory=/usr/local/libexec/yaoe
LimitNOFILE=1048576

[Install]
WantedBy=multi-user.target
UNIT
"#
    } else {
        ""
    };
    let restart_block = if install {
        r#"
systemctl daemon-reload
systemctl enable --now yaoe-sing-box.service
systemctl restart yaoe-sing-box.service
"#
    } else {
        r#"
systemctl restart yaoe-sing-box.service
"#
    };
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
fail() {{ echo "yaoe: $*" >&2; exit 1; }}
download() {{ curl --connect-timeout 10 --max-time 90 -fsSL "$2" -o "$3" || fail "$1 download failed"; }}
[ "$(uname -s)" = "Linux" ] || fail "Linux required"
case "$(uname -m)" in
  x86_64|amd64) arch="amd64" ;;
  aarch64|arm64) arch="arm64" ;;
  *) fail "unsupported CPU architecture" ;;
esac
variant="linux-$arch"
[ "$(id -u)" = "0" ] || fail "root required"
case "${{YAOE_CONFIG_KEY:-}}" in (*[!A-Za-z0-9_-]*|'') fail "YAOE_CONFIG_KEY has invalid shape" ;; esac
[ "${{#YAOE_CONFIG_KEY}}" = "128" ] || fail "YAOE_CONFIG_KEY has invalid length"
{install_block}
/usr/local/libexec/yaoe/sing-box version | grep -F 'sing-box version {SING_BOX_VERSION}' >/dev/null || fail "wrong sing-box version"
config_url="{config_base}/config/$YAOE_CONFIG_KEY/$variant.json"
download "platform config" "$config_url" /etc/yaoe-sing-box/config.json.pending
/usr/local/libexec/yaoe/sing-box check -c /etc/yaoe-sing-box/config.json.pending
{service_block}
mv -f /etc/yaoe-sing-box/config.json.pending /etc/yaoe-sing-box/config.json
{restart_block}
[ "$(systemctl is-active yaoe-sing-box.service)" = "active" ] || fail "service is not active"
"#
    )
}

fn render_macos_script(config: &Config, install: bool) -> String {
    let config_base = config_base(config);
    let release_base = release_base(config);
    let install_block = if install {
        format!(
            r#"
install -d -m 0755 /usr/local/libexec/yaoe
install -d -m 0755 '/Library/Application Support/YAOE/sing-box'
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
archive_url="{release_base}/sing-box-{SING_BOX_VERSION}-macos-$arch.tar.gz"
download "sing-box archive" "$archive_url" "$tmp/sing-box.tar.gz"
tar -xzf "$tmp/sing-box.tar.gz" -C "$tmp"
bin="$(find "$tmp" -type f -name sing-box | head -n 1)"
[ -n "$bin" ] || fail "sing-box not found in archive"
install -m 0755 "$bin" /usr/local/libexec/yaoe/sing-box
"#
        )
    } else {
        r#"
[ -x /usr/local/libexec/yaoe/sing-box ] || fail "missing sing-box executable"
[ -f /Library/LaunchDaemons/io.yaoe.sing-box.plist ] || fail "missing launchd plist"
install -d -m 0755 '/Library/Application Support/YAOE/sing-box'
"#
        .to_string()
    };
    let plist_block = if install {
        r#"
cat > /Library/LaunchDaemons/io.yaoe.sing-box.plist <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>io.yaoe.sing-box</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/libexec/yaoe/sing-box</string>
    <string>run</string>
    <string>-c</string>
    <string>/Library/Application Support/YAOE/sing-box/config.json</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>WorkingDirectory</key>
  <string>/usr/local/libexec/yaoe</string>
</dict>
</plist>
PLIST
chown root:wheel /Library/LaunchDaemons/io.yaoe.sing-box.plist
chmod 0644 /Library/LaunchDaemons/io.yaoe.sing-box.plist
"#
    } else {
        ""
    };
    let restart_block = if install {
        r#"
launchctl print system/io.yaoe.sing-box >/dev/null 2>&1 && launchctl bootout system /Library/LaunchDaemons/io.yaoe.sing-box.plist || true
launchctl bootstrap system /Library/LaunchDaemons/io.yaoe.sing-box.plist
launchctl enable system/io.yaoe.sing-box
"#
    } else {
        r#"
launchctl kickstart -k system/io.yaoe.sing-box
"#
    };
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
fail() {{ echo "yaoe: $*" >&2; exit 1; }}
download() {{ curl --connect-timeout 10 --max-time 90 -fsSL "$2" -o "$3" || fail "$1 download failed"; }}
[ "$(uname -s)" = "Darwin" ] || fail "Darwin required"
case "$(uname -m)" in
  x86_64) arch="amd64" ;;
  arm64) arch="arm64" ;;
  *) fail "unsupported CPU architecture" ;;
esac
variant="macos-$arch"
[ "$(id -u)" = "0" ] || fail "root required"
case "${{YAOE_CONFIG_KEY:-}}" in (*[!A-Za-z0-9_-]*|'') fail "YAOE_CONFIG_KEY has invalid shape" ;; esac
[ "${{#YAOE_CONFIG_KEY}}" = "128" ] || fail "YAOE_CONFIG_KEY has invalid length"
{install_block}
/usr/local/libexec/yaoe/sing-box version | grep -F 'sing-box version {SING_BOX_VERSION}' >/dev/null || fail "wrong sing-box version"
config_url="{config_base}/config/$YAOE_CONFIG_KEY/$variant.json"
download "platform config" "$config_url" '/Library/Application Support/YAOE/sing-box/config.json.pending'
/usr/local/libexec/yaoe/sing-box check -c '/Library/Application Support/YAOE/sing-box/config.json.pending'
{plist_block}
mv -f '/Library/Application Support/YAOE/sing-box/config.json.pending' '/Library/Application Support/YAOE/sing-box/config.json'
{restart_block}
launchctl print system/io.yaoe.sing-box >/dev/null
"#
    )
}
