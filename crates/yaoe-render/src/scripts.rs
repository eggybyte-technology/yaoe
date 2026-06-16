use yaoe_config::Config;
use yaoe_home::{
    GITEE_BOOTSTRAP_BRANCH, GITEE_RELEASE_TAG, HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS,
    HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS, HEALTH_PROBE_URL, IMAGE_CLIENT_REGISTRY, SING_BOX_VERSION,
    YaoeError, YaoeResult, script_extension,
};

pub fn render_install_script(config: &Config, target: &str) -> YaoeResult<String> {
    match target {
        "linux" => Ok(render_linux_script(config, true)),
        "macos" => Ok(render_macos_script(config, true)),
        "linux-image" => Ok(render_linux_image_script(config)),
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

fn render_linux_image_script(config: &Config) -> String {
    let config_base = config_base(config);
    let release_base = release_base(config);
    let amd64 = IMAGE_CLIENT_REGISTRY
        .iter()
        .find(|entry| entry.arch == "amd64")
        .expect("amd64 image registry entry");
    let arm64 = IMAGE_CLIENT_REGISTRY
        .iter()
        .find(|entry| entry.arch == "arm64")
        .expect("arm64 image registry entry");
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
log() {{ printf 'yaoe: %s\n' "$*" >&2; }}
fail() {{ log "ERROR: $*"; exit 1; }}
download() {{
  log "downloading $1 to $3"
  curl --connect-timeout 10 --max-time 90 -fsSL "$2" -o "$3" || fail "$1 download failed"
  log "downloaded $1"
}}
log "starting YAOE sing-box linux image install"
[ "$(uname -s)" = "Linux" ] || fail "Linux required"
[ "$(id -u)" = "0" ] || fail "root required"
case "${{YAOE_CONFIG_KEY:-}}" in (*[!A-Za-z0-9_-]*|'') fail "YAOE_CONFIG_KEY has invalid shape" ;; esac
[ "${{#YAOE_CONFIG_KEY}}" = "128" ] || fail "YAOE_CONFIG_KEY has invalid length"
log "validated root privileges and config key shape"
case "${{YAOE_IMAGE_ARCH:-}}" in
  amd64) arch="amd64"; variant="{amd64_variant}"; runtime_asset="{amd64_asset}" ;;
  arm64) arch="arm64"; variant="{arm64_variant}"; runtime_asset="{arm64_asset}" ;;
  *) fail "YAOE_IMAGE_ARCH must be amd64 or arm64" ;;
esac
log "selected linux image arch=$arch variant=$variant"
log "creating image install directories"
install -d -m 0755 /usr/local/libexec/yaoe
install -d -m 0755 /etc/yaoe-sing-box
install -d -m 0755 /etc/systemd/system
install -d -m 0755 /etc/systemd/system/multi-user.target.wants
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
archive_url="{release_base}/$runtime_asset"
log "downloading sing-box runtime for $variant"
download "sing-box archive" "$archive_url" "$tmp/sing-box.tar.gz"
log "extracting sing-box runtime"
tar -xzf "$tmp/sing-box.tar.gz" -C "$tmp"
bin="$(find "$tmp" -type f -name sing-box | head -n 1)"
[ -n "$bin" ] || fail "sing-box not found in archive"
log "installing sing-box executable to /usr/local/libexec/yaoe/sing-box"
install -m 0755 "$bin" /usr/local/libexec/yaoe/sing-box
config_url="{config_base}/config/$YAOE_CONFIG_KEY/$variant.json"
log "downloading platform config for $variant"
download "platform config" "$config_url" /etc/yaoe-sing-box/config.json.pending
log "activating staged config"
mv -f /etc/yaoe-sing-box/config.json.pending /etc/yaoe-sing-box/config.json
log "rendering systemd unit /etc/systemd/system/yaoe-sing-box.service"
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
log "enabling yaoe-sing-box.service by symlink"
rm -f /etc/systemd/system/multi-user.target.wants/yaoe-sing-box.service
ln -s ../yaoe-sing-box.service /etc/systemd/system/multi-user.target.wants/yaoe-sing-box.service
log "YAOE sing-box linux image install completed: service=enabled"
"#,
        amd64_variant = amd64.config_variant,
        amd64_asset = amd64.public_runtime_asset,
        arm64_variant = arm64.config_variant,
        arm64_asset = arm64.public_runtime_asset,
    )
}

fn render_linux_script(config: &Config, install: bool) -> String {
    let config_base = config_base(config);
    let release_base = release_base(config);
    let operation = if install { "install" } else { "update" };
    let install_block = if install {
        format!(
            r#"
log "creating install directories"
install -d -m 0755 /usr/local/libexec/yaoe
install -d -m 0755 /etc/yaoe-sing-box
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
archive_url="{release_base}/sing-box-{SING_BOX_VERSION}-linux-$arch.tar.gz"
log "downloading sing-box runtime for linux-$arch"
download "sing-box archive" "$archive_url" "$tmp/sing-box.tar.gz"
log "extracting sing-box runtime"
tar -xzf "$tmp/sing-box.tar.gz" -C "$tmp"
bin="$(find "$tmp" -type f -name sing-box | head -n 1)"
[ -n "$bin" ] || fail "sing-box not found in archive"
log "installing sing-box executable to /usr/local/libexec/yaoe/sing-box"
install -m 0755 "$bin" /usr/local/libexec/yaoe/sing-box
"#
        )
    } else {
        r#"
log "checking existing sing-box install artifacts"
[ -x /usr/local/libexec/yaoe/sing-box ] || fail "missing sing-box executable"
[ -f /etc/systemd/system/yaoe-sing-box.service ] || fail "missing service definition"
install -d -m 0755 /etc/yaoe-sing-box
log "existing sing-box install artifacts are present"
"#
        .to_string()
    };
    let service_block = if install {
        r#"
log "rendering systemd unit /etc/systemd/system/yaoe-sing-box.service"
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
log "reloading systemd"
systemctl daemon-reload
log "enabling yaoe-sing-box.service"
systemctl enable --now yaoe-sing-box.service
log "restarting yaoe-sing-box.service"
systemctl restart yaoe-sing-box.service
"#
    } else {
        r#"
log "restarting yaoe-sing-box.service"
systemctl restart yaoe-sing-box.service
"#
    };
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
log() {{ printf 'yaoe: %s\n' "$*" >&2; }}
fail() {{ log "ERROR: $*"; exit 1; }}
download() {{
  log "downloading $1 to $3"
  curl --connect-timeout 10 --max-time 90 -fsSL "$2" -o "$3" || fail "$1 download failed"
  log "downloaded $1"
}}
smoke_probe() {{
  last_result="none"
  for attempt in 1 2 3 4 5; do
    for probe in \
      "https://www.google.com/generate_204|204" \
      "{HEALTH_PROBE_URL}|204" \
      "https://github.com|200" \
      "https://api.github.com/rate_limit|200"
    do
      url="${{probe%|*}}"
      expected="${{probe##*|}}"
      status=""
      curl_exit=0
      log "running service smoke probe attempt $attempt: $url expected_http=$expected"
      if status="$(curl --ipv4 --connect-timeout {HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS} --max-time {HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS} -fsS -o /dev/null -w '%{{http_code}}' "$url")"; then
        if [ "$status" = "$expected" ]; then
          log "service smoke probe OK: url=$url http=$status"
          return 0
        fi
        log "service smoke probe attempt $attempt unexpected status: url=$url http=$status expected_http=$expected"
      else
        curl_exit=$?
        log "service smoke probe attempt $attempt failed: url=$url curl_exit=$curl_exit http=${{status:-000}}"
      fi
      last_result="url=$url curl_exit=$curl_exit http=${{status:-000}} expected_http=$expected"
    done
    sleep 2
  done
  log "WARNING: service smoke probe did not reach public test endpoints; last_result=$last_result"
  return 1
}}
log "starting YAOE sing-box linux {operation}"
[ "$(uname -s)" = "Linux" ] || fail "Linux required"
case "$(uname -m)" in
  x86_64|amd64) arch="amd64" ;;
  aarch64|arm64) arch="arm64" ;;
  *) fail "unsupported CPU architecture" ;;
esac
variant="linux-$arch"
log "detected platform: linux arch=$arch variant=$variant"
[ "$(id -u)" = "0" ] || fail "root required"
case "${{YAOE_CONFIG_KEY:-}}" in (*[!A-Za-z0-9_-]*|'') fail "YAOE_CONFIG_KEY has invalid shape" ;; esac
[ "${{#YAOE_CONFIG_KEY}}" = "128" ] || fail "YAOE_CONFIG_KEY has invalid length"
log "validated root privileges and config key shape"
{install_block}
log "checking sing-box version"
/usr/local/libexec/yaoe/sing-box version | grep -F 'sing-box version {SING_BOX_VERSION}' >/dev/null || fail "wrong sing-box version"
log "sing-box version OK: {SING_BOX_VERSION}"
config_url="{config_base}/config/$YAOE_CONFIG_KEY/$variant.json"
log "downloading platform config for $variant"
download "platform config" "$config_url" /etc/yaoe-sing-box/config.json.pending
log "checking pending sing-box config"
/usr/local/libexec/yaoe/sing-box check -c /etc/yaoe-sing-box/config.json.pending
log "pending sing-box config OK"
{service_block}
log "activating checked config"
mv -f /etc/yaoe-sing-box/config.json.pending /etc/yaoe-sing-box/config.json
{restart_block}
log "waiting for immediate sing-box runtime failures"
sleep 2
service_state="$(systemctl is-active yaoe-sing-box.service || true)"
log "systemd state: yaoe-sing-box.service=$service_state"
[ "$service_state" = "active" ] || fail "service is not active"
if smoke_probe; then smoke_result="ok"; else smoke_result="warning"; fi
log "YAOE sing-box linux {operation} completed: service=active smoke_probe=$smoke_result"
"#
    )
}

fn render_macos_script(config: &Config, install: bool) -> String {
    let config_base = config_base(config);
    let release_base = release_base(config);
    let operation = if install { "install" } else { "update" };
    let install_block = if install {
        format!(
            r#"
log "creating install directories"
install -d -m 0755 /usr/local/libexec/yaoe
install -d -m 0755 '/Library/Application Support/YAOE/sing-box'
install -d -m 0755 /Library/Logs/YAOE
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
archive_url="{release_base}/sing-box-{SING_BOX_VERSION}-macos-$arch.tar.gz"
log "downloading sing-box runtime for macos-$arch"
download "sing-box archive" "$archive_url" "$tmp/sing-box.tar.gz"
log "extracting sing-box runtime"
tar -xzf "$tmp/sing-box.tar.gz" -C "$tmp"
bin="$(find "$tmp" -type f -name sing-box | head -n 1)"
[ -n "$bin" ] || fail "sing-box not found in archive"
log "installing sing-box executable to /usr/local/libexec/yaoe/sing-box"
install -m 0755 "$bin" /usr/local/libexec/yaoe/sing-box
"#
        )
    } else {
        r#"
log "checking existing sing-box install artifacts"
[ -x /usr/local/libexec/yaoe/sing-box ] || fail "missing sing-box executable"
[ -f /Library/LaunchDaemons/io.yaoe.sing-box.plist ] || fail "missing launchd plist"
install -d -m 0755 '/Library/Application Support/YAOE/sing-box'
log "existing sing-box install artifacts are present"
"#
        .to_string()
    };
    let plist_block = if install {
        r#"
log "rendering launchd plist /Library/LaunchDaemons/io.yaoe.sing-box.plist"
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
  <key>StandardOutPath</key>
  <string>/Library/Logs/YAOE/sing-box.out.log</string>
  <key>StandardErrorPath</key>
  <string>/Library/Logs/YAOE/sing-box.err.log</string>
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
log "unloading existing launchd service when present"
launchctl print system/io.yaoe.sing-box >/dev/null 2>&1 && launchctl bootout system /Library/LaunchDaemons/io.yaoe.sing-box.plist || true
log "bootstrapping launchd service"
launchctl bootstrap system /Library/LaunchDaemons/io.yaoe.sing-box.plist
log "enabling launchd service"
launchctl enable system/io.yaoe.sing-box
"#
    } else {
        r#"
log "kickstarting launchd service"
launchctl kickstart -k system/io.yaoe.sing-box
"#
    };
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
log() {{ printf 'yaoe: %s\n' "$*" >&2; }}
fail() {{ log "ERROR: $*"; exit 1; }}
download() {{
  log "downloading $1 to $3"
  curl --connect-timeout 10 --max-time 90 -fsSL "$2" -o "$3" || fail "$1 download failed"
  log "downloaded $1"
}}
smoke_probe() {{
  last_result="none"
  for attempt in 1 2 3 4 5; do
    for probe in \
      "https://www.google.com/generate_204|204" \
      "{HEALTH_PROBE_URL}|204" \
      "https://github.com|200" \
      "https://api.github.com/rate_limit|200"
    do
      url="${{probe%|*}}"
      expected="${{probe##*|}}"
      status=""
      curl_exit=0
      log "running service smoke probe attempt $attempt: $url expected_http=$expected"
      if status="$(curl --ipv4 --connect-timeout {HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS} --max-time {HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS} -fsS -o /dev/null -w '%{{http_code}}' "$url")"; then
        if [ "$status" = "$expected" ]; then
          log "service smoke probe OK: url=$url http=$status"
          return 0
        fi
        log "service smoke probe attempt $attempt unexpected status: url=$url http=$status expected_http=$expected"
      else
        curl_exit=$?
        log "service smoke probe attempt $attempt failed: url=$url curl_exit=$curl_exit http=${{status:-000}}"
      fi
      last_result="url=$url curl_exit=$curl_exit http=${{status:-000}} expected_http=$expected"
    done
    sleep 2
  done
  log "WARNING: service smoke probe did not reach public test endpoints; last_result=$last_result"
  return 1
}}
log "starting YAOE sing-box macos {operation}"
[ "$(uname -s)" = "Darwin" ] || fail "Darwin required"
case "$(uname -m)" in
  x86_64) arch="amd64" ;;
  arm64) arch="arm64" ;;
  *) fail "unsupported CPU architecture" ;;
esac
variant="macos-$arch"
log "detected platform: macos arch=$arch variant=$variant"
[ "$(id -u)" = "0" ] || fail "root required"
case "${{YAOE_CONFIG_KEY:-}}" in (*[!A-Za-z0-9_-]*|'') fail "YAOE_CONFIG_KEY has invalid shape" ;; esac
[ "${{#YAOE_CONFIG_KEY}}" = "128" ] || fail "YAOE_CONFIG_KEY has invalid length"
log "validated root privileges and config key shape"
{install_block}
log "checking sing-box version"
/usr/local/libexec/yaoe/sing-box version | grep -F 'sing-box version {SING_BOX_VERSION}' >/dev/null || fail "wrong sing-box version"
log "sing-box version OK: {SING_BOX_VERSION}"
config_url="{config_base}/config/$YAOE_CONFIG_KEY/$variant.json"
log "downloading platform config for $variant"
download "platform config" "$config_url" '/Library/Application Support/YAOE/sing-box/config.json.pending'
log "checking pending sing-box config"
/usr/local/libexec/yaoe/sing-box check -c '/Library/Application Support/YAOE/sing-box/config.json.pending'
log "pending sing-box config OK"
{plist_block}
log "activating checked config"
mv -f '/Library/Application Support/YAOE/sing-box/config.json.pending' '/Library/Application Support/YAOE/sing-box/config.json'
{restart_block}
log "waiting for immediate sing-box runtime failures"
sleep 2
launchd_state="$(launchctl print system/io.yaoe.sing-box 2>/dev/null | awk -F'= ' '/state =/ {{ print $2; exit }}')"
log "launchd state: io.yaoe.sing-box=${{launchd_state:-unknown}}"
[ "$launchd_state" = "running" ] || fail "service is not running"
if smoke_probe; then smoke_result="ok"; else smoke_result="warning"; fi
log "YAOE sing-box macos {operation} completed: service=running smoke_probe=$smoke_result"
"#
    )
}
