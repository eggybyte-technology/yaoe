# YAOE v0.0.1 Design

## 0. Document Contract

| Field                                   | Value                                                                                                                                                               |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Project                                 | `yaoe`                                                                                                                                                              |
| Product revision                        | `v0.0.1`                                                                                                                                                            |
| License                                 | Apache License 2.0                                                                                                                                                  |
| Authoritative repository file           | `docs/design.md`                                                                                                                                                    |
| Controller command                      | `yaoe`                                                                                                                                                              |
| Controller execution OS                 | Linux                                                                                                                                                               |
| Managed server runtime                  | Linux amd64 or arm64 with systemd                                                                                                                                   |
| Managed server proxy core               | sing-box `1.13.13`                                                                                                                                                  |
| Managed server protocol                 | VLESS over TCP with TLS Reality and Vision flow                                                                                                                     |
| Managed server TLS certificate          | None                                                                                                                                                                |
| Linux desktop service client            | YAOE-installed sing-box systemd service on amd64 and arm64                                                                                                          |
| macOS desktop service client            | YAOE-installed sing-box launchd service on Intel and Apple Silicon                                                                                                  |
| Windows desktop GUI client              | Clash Verge Rev importing one YAOE-generated mihomo profile                                                                                                         |
| macOS desktop GUI client                | Clash Verge Rev importing the same YAOE-generated mihomo profile                                                                                                    |
| Linux desktop GUI client                | Clash Verge Rev importing the same YAOE-generated mihomo profile                                                                                                    |
| Clash Verge Rev reference release       | `v2.5.1`, GitHub Latest stable release dated 2026-05-20                                                                                                             |
| Clash Verge Rev bundled mihomo baseline | mihomo `v1.19.25` from Clash Verge Rev `v2.5.0` release notes                                                                                                       |
| Standalone mihomo validation release    | mihomo `v1.19.27`                                                                                                                                                   |
| Mobile client model                     | Official sing-box graphical clients importing YAOE-generated sing-box Remote Profiles                                                                               |
| Binary and SRS delivery store           | Gitee Release attachments                                                                                                                                           |
| Bootstrap script delivery store         | Gitee repository raw files on branch `main`                                                                                                                         |
| Config delivery store                   | Cloudflare R2 public bucket through one custom domain                                                                                                               |
| Config request credential               | URL path segment equal to `credential.config_key`                                                                                                                   |
| Service desktop config variants         | `linux-amd64`, `linux-arm64`, `macos-amd64`, `macos-arm64`                                                                                                          |
| Mobile config variants                  | `ios`, `android`                                                                                                                                                    |
| GUI config variant                      | `clash-verge`                                                                                                                                                       |
| Published config objects                | `clash-verge.yaml`, `linux-amd64.json`, `linux-arm64.json`, `macos-amd64.json`, `macos-arm64.json`, `ios.json`, `android.json`                                      |
| User-editable configuration             | `.yaoe/yaoe.toml`                                                                                                                                                   |
| Local state root                        | `.yaoe/` relative to the repository root                                                                                                                            |
| Local persistence model                 | Files under `.yaoe/`; no database                                                                                                                                   |
| Rust toolchain                          | Rust `1.96.0`                                                                                                                                                       |
| Rust toolchain file                     | `rust-toolchain.toml`                                                                                                                                               |
| Rust test runner                        | `cargo nextest run`                                                                                                                                                 |
| Development environment                 | Nix flake default devShell loaded by direnv                                                                                                                         |
| Environment entrypoint                  | `.envrc` containing exactly `use flake`                                                                                                                             |
| Client entrypoint command               | `yaoe client`                                                                                                                                                       |
| Runtime health probe                    | Local sing-box `mixed` inbound with SOCKS5 remote hostname resolution probing VLESS/TCP/Reality/Vision without TUN                                                   |
| Client routing model                    | Private/local and configured direct IPv4 CIDRs direct; managed-server endpoint IPv4 `/32` direct; CN allowlist direct; remaining public IPv4 uses proxy aggregation |
| Acceptance validation                   | nextest-controlled controller workflow ending in `yaoe health`                                                                                                      |

This document is the complete engineering contract for YAOE `v0.0.1`. Implementation, repository layout, command surface, configuration file, local state, generated artifacts, Gitee publication, Cloudflare R2 publication, Reality server installation, desktop client entrypoints, mobile client entrypoints, tests, acceptance, logs, and README content are valid when they match this document.

YAOE `v0.0.1` is a single-operator egress and profile-delivery tool. It provisions one or more public Linux amd64 or arm64 egress servers. Each managed server exposes exactly one sing-box VLESS/TCP/Reality/Vision inbound on a configured high TCP port and is installed over root SSH as one systemd service. Linux and macOS service clients install sing-box as a local system service through public YAOE scripts. Windows users import a generated mihomo profile into Clash Verge Rev. macOS and Linux users receive the same Clash Verge Rev profile in addition to the sing-box service entrypoints. iOS and Android users import generated sing-box Remote Profile URLs into official sing-box graphical clients.

### 0.1 Normative Terms

| Term                   | Meaning                                                                                                                                                                                                    |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| MUST                   | Required behavior.                                                                                                                                                                                         |
| REQUIRED               | Required input or behavior.                                                                                                                                                                                |
| EXACT                  | Byte-level, field-level, or sequence-level equality as specified.                                                                                                                                          |
| INVALID                | Rejected before the command performs external side effects.                                                                                                                                                |
| DEFAULT                | Value used when a configuration field is absent.                                                                                                                                                           |
| Repository context     | Repository root on Linux after direnv has loaded the default Nix devShell.                                                                                                                                 |
| Operator               | The single person or automation process that edits `.yaoe/yaoe.toml` and runs `yaoe`.                                                                                                                      |
| Managed server         | One Linux amd64 or arm64 public egress server declared as `[server.<name>]`.                                                                                                                               |
| Desktop service client | A Linux or macOS machine that runs the YAOE sing-box install or update script.                                                                                                                             |
| Desktop GUI client     | A Windows, macOS, or Linux machine running Clash Verge Rev and importing `clash-verge.yaml`.                                                                                                               |
| Mobile client          | An official sing-box graphical client that imports the YAOE `ios.json` or `android.json` Remote Profile URL.                                                                                               |
| Config key             | The value of `credential.config_key` in `.yaoe/yaoe.toml`.                                                                                                                                                 |
| Config object key      | An R2 object key under `config/<credential.config_key>/`.                                                                                                                                                  |
| GUI profile URL        | `https://<cloudflare.delivery_domain>/config/<credential.config_key>/clash-verge.yaml`.                                                                                                                    |
| Clash import URL       | `clash://install-config?url=<percent-encoded-gui-profile-url>`.                                                                                                                                            |
| Service config URL     | `https://<cloudflare.delivery_domain>/config/<credential.config_key>/<service-variant>.json`.                                                                                                              |
| Mobile profile URL     | `https://<cloudflare.delivery_domain>/config/<credential.config_key>/<mobile-variant>.json`.                                                                                                               |
| Reality private key    | The value of `credential.reality_private_key` in `.yaoe/yaoe.toml`.                                                                                                                                        |
| Reality public key     | The X25519 public key derived from `credential.reality_private_key`; it is a derived value.                                                                                                                |
| Reality short ID       | The value of `credential.reality_short_id` in `.yaoe/yaoe.toml`.                                                                                                                                           |
| CN direct SRS          | The two Gitee-published sing-box binary rule-set files `cn-domain.srs` and `cn-ipv4.srs`.                                                                                                                  |
| Effectful command      | A command that changes `.yaoe/`, contacts GitHub, contacts Gitee, contacts Cloudflare, contacts SSH targets, installs or restarts a service, publishes delivery assets, or runs active runtime validation. |
| Atomic write           | Write to a temporary file in the destination directory, flush the file, rename over the destination, and flush the containing directory.                                                                   |

### 0.2 Architecture Decisions

The managed-server protocol is VLESS/TCP/Reality/Vision on sing-box `1.13.13`. The server-side design uses direct IPv4 endpoints and a third-party Reality handshake destination. It requires no server-owned TLS certificate and no server endpoint DNS name.

The desktop service path exists for Linux and macOS. Linux service clients run sing-box under systemd. macOS service clients run sing-box under launchd. The service clients receive architecture-specific sing-box JSON configs from Cloudflare R2 and architecture-specific sing-box runtime artifacts from Gitee Release.

The desktop GUI path uses one mihomo YAML profile named `clash-verge.yaml`. The same profile is valid for Windows, macOS, and Linux Clash Verge Rev clients. YAOE generates the complete mihomo profile from `.yaoe/yaoe.toml`, including VLESS Reality nodes, URLTest aggregation, DNS, TUN settings, geodata settings, and routing rules. Users import the URL or the `clash://install-config` URL scheme; users do not edit rules or node YAML.

The mobile path uses official sing-box graphical clients. YAOE publishes `ios.json` and `android.json` Remote Profile objects to R2. Those profiles use mobile TUN semantics and the same VLESS Reality server facts as every other client.

The link distribution model has three surfaces:

```text
Gitee Release attachments -> sing-box runtime artifacts for Linux/macOS service clients, Linux amd64 and arm64 server runtime fallback, and CN direct SRS files
Gitee repository raw files -> Linux/macOS service install and update scripts
Cloudflare R2 custom-domain objects -> all generated client configs
```

YAOE uses a fixed direct allowlist routing posture. Private/local networks, configured direct IPv4 CIDRs, managed-server endpoint IPv4 `/32` exclusions, and CN allowlist matches route direct. Remaining public IPv4 routes through the `proxy` or `PROXY` aggregation group.

YAOE uses IPv4 managed-server endpoints and IPv4 egress semantics. Desktop GUI mihomo profiles set top-level `ipv6: false` and `dns.ipv6: false`. sing-box service and mobile profiles assign both IPv4 and IPv6 TUN addresses, preserve local/private IPv6 scopes, and reject public IPv6 locally before CN rule evaluation.

### 0.3 Central Constants and Registries

The implementation MUST define one production module containing these constants and registry entries. Validation, rendering, packaging, cache paths, release asset names, README commands, logs, tests, health probes, and acceptance assertions MUST derive from this module.

```text
YAOE_PRODUCT_REVISION = "v0.0.1"
RUST_TOOLCHAIN_VERSION = "1.96.0"
SING_BOX_VERSION = "1.13.13"
SING_BOX_RELEASE_TAG = "v1.13.13"
SING_BOX_ARTIFACT_ROOT = "sing-box/1.13.13"
MIHOMO_VALIDATION_VERSION = "1.19.27"
CLASH_VERGE_REV_REFERENCE_VERSION = "2.5.1"
CLASH_VERGE_REV_REFERENCE_TAG = "v2.5.1"
CLASH_VERGE_REV_REFERENCE_DATE = "2026-05-20"
CLASH_VERGE_MIHOMO_BASELINE_VERSION = "1.19.25"
GITEE_BOOTSTRAP_BRANCH = "main"
GITEE_RELEASE_TAG = "yaoe-v0.0.1-sing-box-1.13.13"
SERVICE_SCRIPT_TARGETS = ["linux", "macos"]
SERVICE_CONFIG_VARIANTS = ["linux-amd64", "linux-arm64", "macos-amd64", "macos-arm64"]
MOBILE_CONFIG_VARIANTS = ["ios", "android"]
GUI_CONFIG_VARIANTS = ["clash-verge"]
CONFIG_VARIANTS = ["clash-verge", "linux-amd64", "linux-arm64", "macos-amd64", "macos-arm64", "ios", "android"]
MANAGED_SERVER_RUNTIME_VARIANTS = ["linux-amd64", "linux-arm64"]
CONFIG_KEY_RANDOM_BYTES = 96
CONFIG_KEY_LENGTH = 128
REALITY_PRIVATE_KEY_LENGTH = 43
REALITY_PUBLIC_KEY_LENGTH = 43
REALITY_SHORT_ID_BYTES = 8
REALITY_SHORT_ID_HEX_LENGTH = 16
SERVER_PORT_MIN = 20000
SERVER_PORT_MAX = 60999
R2_JSON_CONTENT_TYPE = "application/json; charset=utf-8"
R2_YAML_CONTENT_TYPE = "text/yaml; charset=utf-8"
R2_CONFIG_CACHE_CONTROL = "no-store"
CLOUDFLARE_PUBLIC_FETCH_ATTEMPTS = 60
CLOUDFLARE_PUBLIC_FETCH_INTERVAL_SECONDS = 5
HEALTH_PROBE_URL = "https://www.gstatic.com/generate_204"
HEALTH_PROBE_EXPECTED_STATUS = 204
HEALTH_PROBE_CURL_PROXY_KIND = "socks5-remote-resolve"
HEALTH_PROBE_BIND_HOST = "127.0.0.1"
HEALTH_PROBE_STARTUP_TIMEOUT_SECONDS = 3
HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS = 8
HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS = 12
HEALTH_PROBE_PORT_RETRY_LIMIT = 3
REMOTE_JOURNAL_TAIL_LINES = 80
TUN_IPV4_ADDRESS = "172.19.0.1/30"
TUN_IPV6_ADDRESS = "fdfe:dcba:9876::1/126"
PUBLIC_IPV6_DENIAL_NO_DROP = true
SING_BOX_DNS_STRATEGY = "ipv4_only"
SING_BOX_DNS_HIJACK_PORT = 53
SING_BOX_CN_DNS_SERVER = "223.5.5.5"
SING_BOX_CN_DNS_PORT = 853
SING_BOX_CN_DNS_TLS_SERVER_NAME = "dns.alidns.com"
SING_BOX_REMOTE_DNS_SERVER = "1.1.1.1"
SING_BOX_REMOTE_DNS_PORT = 853
SING_BOX_REMOTE_DNS_TLS_SERVER_NAME = "cloudflare-dns.com"
CN_DOMAIN_RULE_TAG = "cn-domain"
CN_IPV4_RULE_TAG = "cn-ipv4"
CN_DOMAIN_PUBLIC_ASSET = "cn-domain.srs"
CN_IPV4_PUBLIC_ASSET = "cn-ipv4.srs"
CN_DOMAIN_UPSTREAM_URL = "https://cdn.jsdelivr.net/gh/Dreista/sing-box-rule-set-cn@rule-set/accelerated-domains.china.conf.srs"
CN_IPV4_UPSTREAM_URL = "https://cdn.jsdelivr.net/gh/Dreista/sing-box-rule-set-cn@rule-set/chnroutes.txt.srs"
MIHOMO_PROFILE_FILE = "clash-verge.yaml"
MIHOMO_MIXED_PORT = 7890
MIHOMO_MODE = "rule"
MIHOMO_LOG_LEVEL = "info"
MIHOMO_IPV6 = false
MIHOMO_ALLOW_LAN = false
MIHOMO_DNS_ENABLE = true
MIHOMO_DNS_IPV6 = false
MIHOMO_DNS_ENHANCED_MODE = "fake-ip"
MIHOMO_FAKE_IP_RANGE = "198.18.0.1/16"
MIHOMO_TUN_ENABLE = true
MIHOMO_TUN_STACK = "mixed"
MIHOMO_TUN_AUTO_ROUTE = true
MIHOMO_TUN_AUTO_DETECT_INTERFACE = true
MIHOMO_TUN_STRICT_ROUTE = true
MIHOMO_TUN_DNS_HIJACK = ["any:53", "tcp://any:53"]
MIHOMO_URL_TEST_URL = "https://www.gstatic.com/generate_204"
MIHOMO_URL_TEST_INTERVAL_SECONDS = 300
MIHOMO_GEODATA_MODE = true
MIHOMO_GEO_AUTO_UPDATE = true
MIHOMO_GEO_UPDATE_INTERVAL_HOURS = 24
MIHOMO_GEOIP_URL = "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat"
MIHOMO_GEOSITE_URL = "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat"
MIHOMO_MMDB_URL = "https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/country.mmdb"
MIHOMO_DEFAULT_NAMESERVER = ["223.5.5.5", "119.29.29.29"]
MIHOMO_NAMESERVER = ["https://dns.alidns.com/dns-query", "https://doh.pub/dns-query"]
MIHOMO_FALLBACK = ["tls://1.1.1.1", "tls://8.8.8.8"]
MIHOMO_FALLBACK_FILTER_GEOIP = true
MIHOMO_FALLBACK_FILTER_GEOIP_CODE = "CN"
```

The service platform registry contains exactly these entries:

| Config variant | Script target | OS detector          | CPU detector                     | Public config file | Public runtime asset                  | Upstream sing-box asset                | Service backend | TUN profile     | Route profile |
| -------------- | ------------- | -------------------- | -------------------------------- | ------------------ | ------------------------------------- | -------------------------------------- | --------------- | --------------- | ------------- |
| `linux-amd64`  | `linux`       | `uname -s == Linux`  | `uname -m` in `x86_64`, `amd64`  | `linux-amd64.json` | `sing-box-1.13.13-linux-amd64.tar.gz` | `sing-box-1.13.13-linux-amd64.tar.gz`  | systemd         | `linux-service` | `service`     |
| `linux-arm64`  | `linux`       | `uname -s == Linux`  | `uname -m` in `aarch64`, `arm64` | `linux-arm64.json` | `sing-box-1.13.13-linux-arm64.tar.gz` | `sing-box-1.13.13-linux-arm64.tar.gz`  | systemd         | `linux-service` | `service`     |
| `macos-amd64`  | `macos`       | `uname -s == Darwin` | `uname -m == x86_64`             | `macos-amd64.json` | `sing-box-1.13.13-macos-amd64.tar.gz` | `sing-box-1.13.13-darwin-amd64.tar.gz` | launchd         | `macos-service` | `service`     |
| `macos-arm64`  | `macos`       | `uname -s == Darwin` | `uname -m == arm64`              | `macos-arm64.json` | `sing-box-1.13.13-macos-arm64.tar.gz` | `sing-box-1.13.13-darwin-arm64.tar.gz` | launchd         | `macos-service` | `service`     |

The generated config registry contains exactly these entries:

| Config variant | Kind            | Public config file | R2 object key                                     | Validation command         |
| -------------- | --------------- | ------------------ | ------------------------------------------------- | -------------------------- |
| `clash-verge`  | desktop GUI     | `clash-verge.yaml` | `config/<credential.config_key>/clash-verge.yaml` | `mihomo -t -f <file>`      |
| `linux-amd64`  | service desktop | `linux-amd64.json` | `config/<credential.config_key>/linux-amd64.json` | `sing-box check -c <file>` |
| `linux-arm64`  | service desktop | `linux-arm64.json` | `config/<credential.config_key>/linux-arm64.json` | `sing-box check -c <file>` |
| `macos-amd64`  | service desktop | `macos-amd64.json` | `config/<credential.config_key>/macos-amd64.json` | `sing-box check -c <file>` |
| `macos-arm64`  | service desktop | `macos-arm64.json` | `config/<credential.config_key>/macos-arm64.json` | `sing-box check -c <file>` |
| `ios`          | mobile          | `ios.json`         | `config/<credential.config_key>/ios.json`         | `sing-box check -c <file>` |
| `android`      | mobile          | `android.json`     | `config/<credential.config_key>/android.json`     | `sing-box check -c <file>` |

The Gitee Release asset registry contains exactly these entries:

| Asset                                 | Source                                                                                                 |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `sing-box-1.13.13-linux-amd64.tar.gz` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-linux-amd64.tar.gz`  |
| `sing-box-1.13.13-linux-arm64.tar.gz` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-linux-arm64.tar.gz`  |
| `sing-box-1.13.13-macos-amd64.tar.gz` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-darwin-amd64.tar.gz` |
| `sing-box-1.13.13-macos-arm64.tar.gz` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-darwin-arm64.tar.gz` |
| `cn-domain.srs`                       | `CN_DOMAIN_UPSTREAM_URL`                                                                               |
| `cn-ipv4.srs`                         | `CN_IPV4_UPSTREAM_URL`                                                                                 |

The Gitee raw script registry contains exactly these entries:

```text
install/linux.sh
update/linux.sh
install/macos.sh
update/macos.sh
```

### 0.4 Derived Values

The implementation MUST derive these values from `.yaoe/yaoe.toml` and constants:

| Derived value                   | Rule                                                                                                                              |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Reality public key              | X25519 public key derived from `credential.reality_private_key`.                                                                  |
| Config base URL                 | `https://<cloudflare.delivery_domain>`.                                                                                           |
| Gitee bootstrap branch          | `GITEE_BOOTSTRAP_BRANCH`.                                                                                                         |
| Gitee Release tag               | `GITEE_RELEASE_TAG`.                                                                                                              |
| Gitee raw script URLs           | `https://gitee.com/<gitee.owner>/<gitee.repo>/raw/main/<script-path>`.                                                            |
| Gitee Release asset URLs        | `https://gitee.com/<gitee.owner>/<gitee.repo>/releases/download/yaoe-v0.0.1-sing-box-1.13.13/<asset-name>`.                       |
| Config object keys              | For every entry in `CONFIG_VARIANTS`: `config/<credential.config_key>/<public-config-file>`.                                      |
| Config URLs                     | For every entry in `CONFIG_VARIANTS`: `https://<cloudflare.delivery_domain>/config/<credential.config_key>/<public-config-file>`. |
| Desktop GUI profile URL         | `https://<cloudflare.delivery_domain>/config/<credential.config_key>/clash-verge.yaml`.                                           |
| Clash import URL                | `clash://install-config?url=<percent-encoded-desktop-gui-profile-url>`.                                                           |
| Service script paths            | For every entry in `SERVICE_SCRIPT_TARGETS`: `install/<target>.<ext>` and `update/<target>.<ext>`, where `<ext>` is `sh`.         |
| Server outbound tags            | `egress-<server-name>`.                                                                                                           |
| Cloudflare zone ID              | Longest accessible Cloudflare zone suffix for `cloudflare.delivery_domain`.                                                       |
| Health probe mixed inbound bind | `127.0.0.1:<probe-port>`, where `<probe-port>` is selected by the controller for one probe execution.                             |

Reality public key derivation is exact: decode `credential.reality_private_key` as base64url without padding into 32 bytes, compute the X25519 public key using the decoded private scalar and the Curve25519 basepoint, then encode the 32-byte public key as base64url without padding. The result MUST match `^[A-Za-z0-9_-]{43}$`. A derivation failure exits with code `3` before external side effects.

### 0.5 Terminal Output, Logs, Color, and Credential Disclosure

Provider secrets are:

```text
cloudflare.token
gitee.token
SSH private key file contents
```

Every output surface redacts provider secrets. Output surfaces include success output, error output, logs, generated scripts, Git remote URLs, local marker files, temporary command strings, child-process argument vectors, and test fixture golden outputs.

Delivery credentials are:

```text
credential.config_key
full config URLs containing credential.config_key
credential.vless_uuid
credential.reality_private_key
credential.reality_short_id
derived Reality public key
```

YAOE prints delivery credentials only in stdout contracts that expose client entrypoints or generate credentials. The stdout contract is:

```text
yaoe client
  stdout: exactly the client entrypoint block in section 5.4

yaoe publish config
  stdout: one line per config variant after public validation succeeds:
  config <variant> <full-config-url>

yaoe publish delivery
  stdout: the same config lines after the config publication sub-step succeeds

yaoe init, when it creates .yaoe/yaoe.toml
  stdout includes:
  config_key <credential.config_key>
  vless_uuid <credential.vless_uuid>
  reality_private_key <credential.reality_private_key>
  reality_public_key <derived Reality public key>
  reality_short_id <credential.reality_short_id>

yaoe rotate config-key
  stdout includes:
  config_key <new credential.config_key>
  next yaoe publish config

yaoe rotate vless-uuid
  stdout includes:
  vless_uuid <new credential.vless_uuid>
  next yaoe apply
  next yaoe publish config

yaoe rotate reality-keypair
  stdout includes:
  reality_private_key <new credential.reality_private_key>
  reality_public_key <derived Reality public key>
  reality_short_id <new credential.reality_short_id>
  next yaoe apply
  next yaoe publish config
```

All progress, cache, skip, remote, and health-probe diagnostics go to stderr. Stderr logs are one human-readable line per event and use this grammar:

```text
<timestamp> <level> <scope> <message> [<field>=<value> ...]
```

| Element       | Contract                                                                                              |
| ------------- | ----------------------------------------------------------------------------------------------------- |
| `<timestamp>` | RFC3339 UTC timestamp with millisecond precision and `Z` suffix. Example: `2026-06-10T06:40:12.483Z`. |
| `<level>`     | Exactly one of `INFO`, `OK`, `WARN`, `ERROR`.                                                         |
| `<scope>`     | `<command>` or `<command>.<stage>` or `<command>.<stage>:<target>`.                                   |
| `<message>`   | Lowercase human-readable event phrase without trailing punctuation.                                   |
| `<field>`     | `^[a-z][a-z0-9_]*$`.                                                                                  |
| `<value>`     | Unquoted when it contains only `[A-Za-z0-9._:/@%+=,-]`; otherwise JSON-string quoted.                 |

The shared logging helper normalizes messages into one lowercase line with no trailing punctuation and guarantees emitted field keys satisfy the field grammar. Invalid field keys are emitted under `field` with their original key embedded in the value.

Color is automatic. Color is enabled when stderr is a terminal, `TERM` is present, `TERM != dumb`, and `NO_COLOR` is unset. When color is enabled, YAOE colors only the `<level>` token and `<scope>` token. Color codes are exact:

| Token     | ANSI SGR |
| --------- | -------- |
| `INFO`    | `36`     |
| `OK`      | `32`     |
| `WARN`    | `33`     |
| `ERROR`   | `31`     |
| `<scope>` | `1`      |
| reset     | `0`      |

## 1. Fixed Architecture

### 1.1 Purpose

YAOE `v0.0.1` maintains public Linux amd64 or arm64 egress servers and publishes client profiles. It turns one operator-edited `.yaoe/yaoe.toml` into:

1. Managed sing-box server configs and server packages.
2. A complete Clash Verge Rev mihomo YAML profile for Windows, macOS, and Linux GUI users.
3. sing-box service JSON configs for Linux and macOS service users.
4. sing-box Remote Profile JSON configs for iOS and Android users.
5. Public install/update scripts for Linux and macOS service users.
6. Public runtime and CN direct rule-set assets for service and mobile profiles.

The service delivery surfaces are exactly:

```text
Gitee Release attachments -> public sing-box packages and public CN direct SRS files
Gitee repository raw files -> public Linux/macOS install scripts and public Linux/macOS update scripts
Cloudflare R2 public bucket -> config-key path protected generated client configs
```

The generated config variants are exactly:

```text
clash-verge
linux-amd64
linux-arm64
macos-amd64
macos-arm64
ios
android
```

Every config variant receives a variant-specific profile from Cloudflare R2. The `clash-verge` profile is one mihomo YAML profile consumed by Clash Verge Rev on Windows, macOS, and Linux. The four Linux/macOS service profiles are sing-box JSON configs. The two mobile profiles are sing-box JSON Remote Profiles.

### 1.2 Product Behaviors

YAOE `v0.0.1` implements exactly these behaviors:

1. Linux CLI named `yaoe` in repository context.
2. Rust workspace pinned to Rust `1.96.0`.
3. Nix flake, committed `flake.lock`, default devShell, and committed `.envrc` containing exactly `use flake`.
4. `.yaoe/yaoe.toml` as the only user-editable YAOE configuration file.
5. Fixed `.yaoe/` local state root.
6. One or more public Linux amd64 or arm64 egress servers declared as `[server.<name>]`.
7. Shared VLESS UUID stored in `.yaoe/yaoe.toml` and used for every server, every generated profile, and every health probe.
8. Shared Reality private key and short ID stored in `.yaoe/yaoe.toml` and used for every server and every generated profile.
9. Reality public key derived from the private key and used in generated client and probe configs.
10. Shared config key stored in `.yaoe/yaoe.toml` and used as the only config object path credential.
11. `yaoe client` as the exact command for printing GUI, mobile, Linux service, and macOS service entrypoints.
12. Runtime sync of sing-box `1.13.13` artifacts for Linux amd64, Linux arm64, macOS amd64, and macOS arm64 into a Gitee Release.
13. Runtime sync of CN direct SRS files into the same Gitee Release as `cn-domain.srs` and `cn-ipv4.srs`.
14. Rendering and publication of four public service scripts: install and update for Linux, install and update for macOS.
15. Rendering and publication of seven config objects to Cloudflare R2.
16. Linux service install scripts that detect CPU architecture, install the matching sing-box binary, install service definition, install the matching config, and start the systemd service.
17. Linux service update scripts that detect CPU architecture, replace only the matching local config, and restart the existing systemd service.
18. macOS service install scripts that detect CPU architecture, install the matching sing-box binary, install launchd plist, install the matching config, and start the launchd service.
19. macOS service update scripts that detect CPU architecture, replace only the matching local config, and restart the existing launchd service.
20. Desktop GUI config delivery through Clash Verge Rev Remote Profile URL and URL Scheme import URL.
21. Mobile config delivery through official sing-box graphical-client Remote Profile URLs.
22. Server status command through root SSH.
23. Server health command through root SSH plus a local sing-box `mixed` inbound active Reality probe.
24. Explicit local credential rotation commands for config key, VLESS UUID, and Reality keypair.
25. Functional tests colocated with owning modules and crates.
26. Real acceptance validation orchestrated by nextest as a controller workflow ending in `yaoe health`.

### 1.3 Fixed Data Flow

Server deployment:

```text
yaoe apply [<server>]
  -> load and validate .yaoe/yaoe.toml
  -> for each selected managed server, detect remote Linux CPU architecture over root SSH
  -> map amd64 servers to linux-amd64 and arm64 servers to linux-arm64
  -> resolve the matching sing-box 1.13.13 server runtime into .yaoe/cache/server-runtime/
  -> render one Reality server sing-box config per selected managed server
  -> assemble one server package per selected managed server with the matching runtime
  -> upload package over root SSH
  -> run target install.sh as root
  -> require managed systemd service active state
```

Runtime publication:

```text
yaoe publish runtime
  -> load and validate .yaoe/yaoe.toml
  -> ensure the Gitee delivery repository exists
  -> ensure branch main exists with YAOE service scripts when the repository has no main branch
  -> ensure the fixed Gitee Release exists
  -> fetch or reuse four sing-box service runtime artifacts
  -> fetch or reuse two CN direct SRS files
  -> publish missing Gitee Release assets using local marker semantics
```

Bootstrap publication:

```text
yaoe publish bootstrap
  -> load and validate .yaoe/yaoe.toml
  -> ensure the Gitee delivery repository exists
  -> render four Linux/macOS service scripts
  -> ensure branch main exists with YAOE service scripts when the repository has no main branch
  -> publish changed scripts to Gitee branch main using local marker semantics
```

Config publication:

```text
yaoe publish config
  -> load and validate .yaoe/yaoe.toml
  -> require sing-box 1.13.13 from PATH
  -> require mihomo 1.19.27 from PATH
  -> resolve the Cloudflare zone ID for cloudflare.delivery_domain
  -> ensure the configured R2 bucket exists
  -> ensure cloudflare.delivery_domain is connected to the configured R2 bucket
  -> render seven config objects
  -> validate six sing-box JSON configs with sing-box 1.13.13
  -> validate one mihomo YAML config with mihomo 1.19.27
  -> PUT exactly seven config objects to R2 under config/<credential.config_key>/
  -> fetch every public config URL through cloudflare.delivery_domain
  -> validate every fetched config with its matching validator
  -> print full config URLs for all seven config variants
```

Aggregate delivery publication:

```text
yaoe publish delivery
  -> yaoe publish bootstrap
  -> yaoe publish runtime
  -> yaoe publish config
```

Client entrypoint derivation:

```text
yaoe client
  -> parse .yaoe/yaoe.toml
  -> validate fields needed to derive entrypoint URLs and script commands
  -> print Clash Verge Rev entrypoints, mobile Remote Profile URLs, Linux service commands, and macOS service commands
```

Desktop GUI import:

```text
operator runs yaoe client
  -> operator copies the clash-verge remote-profile URL or clash-verge import URL
  -> desktop user imports the profile in Clash Verge Rev
  -> Clash Verge Rev downloads https://<cloudflare.delivery_domain>/config/<credential.config_key>/clash-verge.yaml
  -> mihomo runs VLESS/TCP/Reality/Vision with generated DNS, TUN, geodata, URLTest, and route rules
```

Linux/macOS service install:

```text
operator obtains OS-level command from yaoe client
  -> operator supplies YAOE_CONFIG_KEY in the shell environment exactly as printed
  -> desktop service client runs the public OS-level install script from Gitee raw URL
  -> script validates OS, privilege, YAOE_CONFIG_KEY syntax, and local CPU architecture
  -> script derives the service config variant from OS and CPU architecture
  -> script downloads the matching sing-box runtime from Gitee Release
  -> script downloads config from https://<cloudflare.delivery_domain>/config/$YAOE_CONFIG_KEY/<variant>.json
  -> script writes pending config
  -> script runs sing-box check against pending config
  -> script installs or replaces the local service definition
  -> script atomically promotes config
  -> script starts or restarts the local service
  -> script requires active/running service state
```

Mobile remote profile import:

```text
operator runs yaoe client
  -> operator copies the ios or android Remote Profile URL
  -> operator creates a Remote Profile in the official sing-box graphical client
  -> official client downloads https://<cloudflare.delivery_domain>/config/<credential.config_key>/<variant>.json
  -> official client runs the platform-specific VPN or NetworkExtension implementation
```

Runtime health validation:

```text
yaoe health [<server>]
  -> load and validate .yaoe/yaoe.toml
  -> require local sing-box 1.13.13 from PATH
  -> perform status validation for each selected server over SSH
  -> render one temporary local probe config per selected server
  -> start local sing-box with a mixed inbound on 127.0.0.1:<probe-port>
  -> curl with --ipv4 and --socks5-hostname 127.0.0.1:<probe-port> to HEALTH_PROBE_URL
  -> require HTTP HEALTH_PROBE_EXPECTED_STATUS
  -> stop the local sing-box probe process
```

## 2. Development Environment and Repository Contract

### 2.1 Rust Toolchain Pin

The repository root contains:

```text
Cargo.toml
Cargo.lock
LICENSE
rust-toolchain.toml
flake.nix
flake.lock
.envrc
.config/nextest.toml
README.md
docs/design.md
```

`rust-toolchain.toml` is the authoritative Rust toolchain declaration:

```toml
[toolchain]
channel = "1.96.0"
profile = "minimal"
components = ["rustfmt", "clippy"]
```

The workspace `Cargo.toml` sets:

```toml
[workspace.package]
rust-version = "1.96.0"
license = "Apache-2.0"
```

Inside repository context, `rustc --version` prints a version string that starts with:

```text
rustc 1.96.0
```

### 2.2 Nix Flake, devShell, and direnv

`.envrc` content is exactly:

```bash
use flake
```

A developer runs this once from the repository root:

```bash
direnv allow
```

After direnv loads the repository environment, documented repository commands are invoked directly from the repository root:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo install --path crates/yaoe-cli --locked --force
yaoe check
yaoe client
yaoe publish bootstrap
yaoe publish runtime
yaoe publish config
yaoe publish delivery
yaoe apply
yaoe status
yaoe health
```

The default devShell provides these tools on `PATH`:

```text
rustc
cargo
rustfmt
clippy-driver
cargo-nextest
git
ssh
scp
ssh-keygen
tar
gzip
unzip
jq
sing-box
mihomo
wrangler
cmp
curl
```

`sing-box version` reports `1.13.13` when repository validation, generated sing-box config validation, or health probe validation invokes it. `mihomo -v` reports `1.19.27` when generated mihomo config validation invokes it.

`yaoe init` and `yaoe rotate reality-keypair` invoke `sing-box generate reality-keypair` from `PATH` and require successful output containing one private key and one public key. YAOE writes only the private key to `.yaoe/yaoe.toml`.

`yaoe publish config` and `yaoe publish delivery` invoke `wrangler` from `PATH` and provide these environment variables to the child process from `.yaoe/yaoe.toml`:

```text
CLOUDFLARE_ACCOUNT_ID=<cloudflare.account_id>
CLOUDFLARE_API_TOKEN=<cloudflare.token>
```

### 2.3 Repository Layout and Crate Responsibilities

The committed repository layout is:

```text
.
├── Cargo.toml
├── Cargo.lock
├── LICENSE
├── rust-toolchain.toml
├── flake.nix
├── flake.lock
├── .envrc
├── .config/
│   └── nextest.toml
├── README.md
├── docs/
│   └── design.md
└── crates/
    ├── yaoe-cli/
    ├── yaoe-config/
    ├── yaoe-home/
    ├── yaoe-cloudflare/
    ├── yaoe-gitee/
    ├── yaoe-render/
    ├── yaoe-package/
    ├── yaoe-ssh/
    ├── yaoe-rules/
    ├── yaoe-upstream/
    ├── yaoe-controller/
    │   └── tests/
    │       └── acceptance_delivery.rs
    └── yaoe-server-installer/
```

| Crate                   | Responsibility                                                                                                                                                                                                                                                            |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoe-cli`              | Argument parsing, command dispatch, stdout contracts, stderr log formatting, ANSI color policy, exit-code mapping.                                                                                                                                                        |
| `yaoe-config`           | `.yaoe/yaoe.toml` parsing, defaults, normalization, validation, client-entrypoint validation, Reality public key derivation, and credential file updates for rotate commands.                                                                                             |
| `yaoe-home`             | Fixed `.yaoe/` paths for cache, state, work files, generated delivery workspaces, health probe work files, server packages, Gitee worktrees, and acceptance workspace.                                                                                                    |
| `yaoe-cloudflare`       | Cloudflare zone resolution, R2 bucket orchestration, R2 custom-domain orchestration, Wrangler orchestration, public R2 URL construction, and public config fetch validation.                                                                                              |
| `yaoe-gitee`            | Gitee repository existence, repository creation, branch `main` baseline creation, service script publication, release lookup, release creation, release asset lookup, release asset upload, local publication marker handling, and public URL construction.               |
| `yaoe-upstream`         | Upstream artifact fetching for sing-box service/runtime assets using the platform registry.                                                                                                                                                                               |
| `yaoe-rules`            | CN direct SRS HTTPS fetching, byte-for-byte mirroring to Gitee release assets, fixed public names.                                                                                                                                                                        |
| `yaoe-render`           | Platform registry, Reality sing-box server JSON, Linux/macOS service sing-box JSON, mobile sing-box JSON, Clash Verge Rev mihomo YAML, health probe JSON, Linux/macOS install scripts, Linux/macOS update scripts, systemd units, launchd plists, server install scripts. |
| `yaoe-package`          | Server transfer package assembly.                                                                                                                                                                                                                                         |
| `yaoe-ssh`              | Root SSH/SCP execution, remote reads, remote status commands, remote journal tail collection.                                                                                                                                                                             |
| `yaoe-controller`       | User command workflows across config, home, upstream, Gitee, Cloudflare R2, rendering, packaging, SSH, delivery publication, client entrypoint derivation, rotation, status, health, and acceptance orchestration.                                                        |
| `yaoe-server-installer` | Target-side Linux managed-server installer script generation.                                                                                                                                                                                                             |

`yaoe-controller` keeps command workflows in `lib.rs` and isolates cross-cutting controller concerns in internal modules:

| Module      | Responsibility                                                                                 |
| ----------- | ---------------------------------------------------------------------------------------------- |
| `deps`      | Runtime dependency assembly and command-scoped no-op adapters.                                 |
| `logging`   | Controller-level event naming, progress normalization, and structured log helper entrypoints.   |
| `system`    | Local sing-box, mihomo, curl probe, public-config fetch, and Reality keypair process adapters. |

Test placement rules:

1. Module-private functional tests live in the owning Rust module under `#[cfg(test)]`.
2. Crate public-contract tests live under `crates/<crate>/tests/` when the crate has a committed integration test target.
3. Cross-crate command workflow tests live under `crates/yaoe-controller/tests/`.
4. Acceptance validation lives under `crates/yaoe-controller/tests/acceptance_delivery.rs`.
5. Test helpers and static inputs live under the crate that owns the tested behavior.

Production error handling rules:

1. User input, provider responses, external command output, file-system state, and registry lookups reachable from a command return `YaoeResult` errors instead of panicking.
2. `panic!`, `unwrap`, and `expect` are valid only in tests or after an implementation-owned constant has already been selected from a fixed registry.
3. Command orchestration logs are emitted through the shared logging helper, not with ad hoc stderr writes.

File formats:

| Purpose                             | Format                      |
| ----------------------------------- | --------------------------- |
| User configuration                  | TOML 1.0.0                  |
| Mirrored rule set                   | sing-box binary `.srs`      |
| Server config                       | sing-box JSON               |
| Linux/macOS service config response | sing-box JSON               |
| Mobile config response              | sing-box JSON               |
| Clash Verge Rev config response     | mihomo YAML                 |
| Health probe config                 | sing-box JSON               |
| Server package                      | gzip-compressed tar archive |
| Linux and macOS scripts             | POSIX shell                 |
| Timestamps in human output          | RFC3339 UTC with `Z` suffix |

## 3. Configuration Contract

### 3.1 Configuration File

The only user-editable YAOE configuration file is:

```text
.yaoe/yaoe.toml
```

Allowed root tables are exactly:

```text
ssh
cloudflare
gitee
credential
reality
route
server
```

### 3.2 Complete Example

All values shown below are placeholders except table names, field names, fixed generated credential shapes, and generated ports:

```toml
[ssh]
key = "~/.ssh/id_ed25519"

[cloudflare]
token = "cf_real_cloudflare_api_token"
account_id = "cf_real_account_id"
delivery_domain = "cfg.example.com"
r2_bucket = "yaoe-config"

[gitee]
token = "gitee_real_token"
owner = "your-org"
repo = "yaoe-delivery"

[credential]
vless_uuid = "00000000-0000-4000-8000-000000000000"
config_key = "replace_with_exactly_128_base64url_chars_without_padding_or_slash"
reality_private_key = "replace_with_43_char_sing_box_reality_private_key"
reality_short_id = "0123456789abcdef"

[reality]
handshake_server = "www.cloudflare.com"

[route]
direct_cidrs = ["100.64.0.0/10"]

[server.hk]
ssh = "root@203.0.113.20"
ip = "203.0.113.20"
port = 28443

[server.jp]
ssh = "root@203.0.113.30"
ip = "203.0.113.30"
port = 35443
```

### 3.3 Tables and Fields

`[ssh]` fields:

| Field | Type        | Presence                                   | Meaning                  |
| ----- | ----------- | ------------------------------------------ | ------------------------ |
| `key` | path string | required unless every server defines `key` | Default SSH private key. |

SSH key paths are absolute or start with `~/`. Empty strings, whitespace-only strings, control characters, and shell metacharacters are INVALID.

`[cloudflare]` fields:

| Field             | Type           | Presence | Meaning                                                                                                                               |
| ----------------- | -------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `token`           | string         | required | Cloudflare API token used for zone read, R2 bucket, R2 object, R2 custom domain, and DNS operations required by the R2 custom domain. |
| `account_id`      | string         | required | Cloudflare account ID for R2 operations.                                                                                              |
| `delivery_domain` | FQDN           | required | Public custom hostname connected to the R2 bucket.                                                                                    |
| `r2_bucket`       | R2 bucket name | required | R2 bucket that stores generated config objects.                                                                                       |

`cloudflare.delivery_domain` is a lowercase ASCII FQDN without trailing dot. It contains at least three labels. During `publish config`, Cloudflare must contain exactly one resolvable zone suffix for the delivery domain after longest-suffix selection, and the delivery domain must be exactly one label below that selected zone.

`cloudflare.r2_bucket` matches:

```text
^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$
```

`[gitee]` fields:

| Field   | Type   | Presence | Meaning                                                                                                         |
| ------- | ------ | -------- | --------------------------------------------------------------------------------------------------------------- |
| `token` | string | required | Gitee token used for repository creation, repository branch writes, release creation, and release asset writes. |
| `owner` | string | required | Gitee user or organization namespace.                                                                           |
| `repo`  | string | required | Gitee repository name.                                                                                          |

YAOE creates missing delivery repositories as public repositories. YAOE writes service scripts to branch `main`. YAOE writes release assets to release tag `yaoe-v0.0.1-sing-box-1.13.13`.

`[credential]` fields:

| Field                 | Type                                | Presence | Meaning                                                                     |
| --------------------- | ----------------------------------- | -------- | --------------------------------------------------------------------------- |
| `vless_uuid`          | UUID string                         | required | Shared VLESS UUID for every server and every generated client/probe config. |
| `config_key`          | base64url string                    | required | Shared R2 config path credential.                                           |
| `reality_private_key` | sing-box Reality private key string | required | Server-side Reality private key; client-side public key is derived.         |
| `reality_short_id`    | lowercase hex string                | required | Shared Reality short ID.                                                    |

`credential.config_key` matches exactly:

```text
^[A-Za-z0-9_-]{128}$
```

`credential.reality_private_key` matches exactly:

```text
^[A-Za-z0-9_-]{43}$
```

`credential.reality_short_id` matches exactly:

```text
^[0-9a-f]{16}$
```

`[reality]` fields:

| Field              | Type     | Presence  | Meaning                                                                           |
| ------------------ | -------- | --------- | --------------------------------------------------------------------------------- |
| `handshake_server` | FQDN     | required  | Reality handshake destination and client TLS `server_name` / mihomo `servername`. |
| `handshake_port`   | TCP port | defaulted | Reality handshake destination port.                                               |

DEFAULT `handshake_port`:

```text
443
```

`reality.handshake_server` is a lowercase ASCII FQDN without trailing dot.

`[route]` fields:

| Field          | Type                  | Presence  | Meaning                                                               |
| -------------- | --------------------- | --------- | --------------------------------------------------------------------- |
| `direct_cidrs` | array of CIDR strings | defaulted | Additional IPv4 CIDRs routed direct in every generated client config. |

DEFAULT `direct_cidrs`:

```text
100.64.0.0/10
```

`[server.<name>]` fields:

| Field  | Type                     | Presence                 | Meaning                                                                |
| ------ | ------------------------ | ------------------------ | ---------------------------------------------------------------------- |
| `ssh`  | root OpenSSH destination | required                 | Root SSH target.                                                       |
| `ip`   | IPv4 literal             | required                 | Public server endpoint IPv4 and generated client VLESS `server` value. |
| `port` | TCP port                 | required                 | VLESS/TCP/Reality listen port.                                         |
| `key`  | path string              | defaulted by `[ssh].key` | Server-specific SSH key override.                                      |

Server names match:

```text
^[a-z][a-z0-9-]{0,62}$
```

`server.<name>.port` is in the inclusive range `20000..=60999`. `ssh` begins with `root@`.

### 3.4 Full Validation Rules

The controller applies full validation before every effectful command except `init` and rotate subcommands:

1. Allowed root tables are exactly `ssh`, `cloudflare`, `gitee`, `credential`, `reality`, `route`, and `server`.
2. Allowed fields are exactly the fields in section 3.3.
3. At least one `[server.<name>]` table exists.
4. Server names are valid and unique.
5. `[ssh].key` exists unless every server defines `key`.
6. SSH destinations are root-only and unique.
7. Endpoint IP literals are valid IPv4 literals and unique.
8. Paths are absolute or start with `~/`.
9. Domain values are lowercase ASCII FQDNs without trailing dots.
10. `cloudflare.delivery_domain` has at least three labels.
11. `cloudflare.r2_bucket` is a valid bucket name.
12. `gitee.owner` and `gitee.repo` are non-empty ASCII identifiers without whitespace, slash at either end, control characters, or shell metacharacters.
13. `gitee.token` is not empty or whitespace-only.
14. `reality.handshake_server` is a valid lowercase ASCII FQDN and not an IP literal.
15. `reality.handshake_port`, when present, is an integer in `1..=65535`.
16. Every `server.<name>.port` is an integer in `20000..=60999`.
17. `route.direct_cidrs` values are valid IPv4 CIDRs.
18. Duplicate values inside user-provided `route.direct_cidrs` after canonicalization are INVALID.
19. Cloudflare token, account ID, delivery domain, and R2 bucket are not empty or whitespace-only.
20. `credential.vless_uuid` is a valid UUID string.
21. `credential.config_key` matches section 3.3 exactly.
22. Reality credential fields match section 3.3 exactly.
23. Reality public key derivation from `credential.reality_private_key` succeeds.
24. Placeholder values created by `yaoe init` are INVALID for effectful commands.
25. Values derived by section 0.4 are absent from user configuration.

### 3.5 Client Entrypoint Validation Rules

`yaoe client` applies this validation profile:

1. `.yaoe/yaoe.toml` parses as TOML 1.0.0.
2. Allowed root tables are exactly `ssh`, `cloudflare`, `gitee`, `credential`, `reality`, `route`, and `server`.
3. Allowed fields are exactly the fields in section 3.3.
4. `[cloudflare].delivery_domain` is required, is a lowercase ASCII FQDN without trailing dot, contains at least three labels, and is not an init placeholder.
5. `[credential].config_key` is required, matches `^[A-Za-z0-9_-]{128}$`, and is not an init placeholder.
6. `[gitee].owner` is required, satisfies section 3.4 rule 12, and is not an init placeholder.
7. `[gitee].repo` is required, satisfies section 3.4 rule 12, and is not an init placeholder.
8. Other present fields parse according to schema.

## 4. Local State, Cache, and Generated Workspaces

### 4.1 Home Layout

```text
.yaoe/
├── yaoe.toml
├── cache/
│   ├── upstream/
│   │   ├── sing-box/1.13.13/
│   │   │   ├── linux-amd64/sing-box-1.13.13-linux-amd64.tar.gz
│   │   │   ├── linux-arm64/sing-box-1.13.13-linux-arm64.tar.gz
│   │   │   ├── macos-amd64/sing-box-1.13.13-macos-amd64.tar.gz
│   │   │   └── macos-arm64/sing-box-1.13.13-macos-arm64.tar.gz
│   │   └── srs/{cn-domain.srs,cn-ipv4.srs}
│   ├── server-runtime/sing-box/1.13.13/linux-amd64/sing-box
│   ├── server-runtime/sing-box/1.13.13/linux-arm64/sing-box
│   ├── gitee-work/<gitee.owner>/<gitee.repo>/main/
│   └── published/
│       ├── gitee-release/yaoe-v0.0.1-sing-box-1.13.13/<asset-name>.ok
│       └── gitee-repo/main/<path>.last
└── work/
    ├── delivery/
    │   ├── gitee-repo/
    │   │   ├── install/{linux.sh,macos.sh}
    │   │   └── update/{linux.sh,macos.sh}
    │   └── rendered-config/
    │       ├── clash-verge.yaml
    │       ├── linux-amd64.json
    │       ├── linux-arm64.json
    │       ├── macos-amd64.json
    │       ├── macos-arm64.json
    │       ├── ios.json
    │       └── android.json
    ├── packages/
    ├── health/
    │   └── <server>/probe.json
    └── acceptance/
```

`.yaoe/work/delivery/rendered-config/*` files embed delivery credentials and server facts. These files are generated work files overwritten by `yaoe publish config` and `yaoe publish delivery`.

`.yaoe/work/health/<server>/probe.json` is a generated active-probe config. It is overwritten by `yaoe health`.

`.yaoe/cache/published/` files are local upload-skip cache files. Deleting a file under `.yaoe/cache/published/` forces the corresponding upload path to run again.

### 4.2 `init`

`yaoe init` creates missing directories and a missing `.yaoe/yaoe.toml` sample. Existing `.yaoe/yaoe.toml` remains unchanged.

When `yaoe init` creates `.yaoe/yaoe.toml`, it generates syntactically valid non-placeholder values for:

```text
credential.vless_uuid
credential.config_key
credential.reality_private_key
credential.reality_short_id
server.<sample>.port
```

`credential.config_key` is generated as exactly 128 base64url characters without padding. `credential.reality_private_key` is generated by invoking `sing-box generate reality-keypair`; the public key output is discarded after confirming that local derivation from the private key produces the same public key. `credential.reality_short_id` is generated as exactly 16 lowercase hex characters from 8 random bytes. `server.<sample>.port` is generated uniformly from the inclusive range `20000..=60999`.

### 4.3 Credential Rotation Commands

Credential rotation commands update `.yaoe/yaoe.toml` atomically and contact no network service.

Commands are exactly:

```text
yaoe rotate config-key
yaoe rotate vless-uuid
yaoe rotate reality-keypair
```

`yaoe rotate config-key` updates only `credential.config_key`. It generates exactly 128 base64url characters without padding and prints:

```text
config_key <new credential.config_key>
next yaoe publish config
```

`yaoe rotate vless-uuid` updates only `credential.vless_uuid`. It prints:

```text
vless_uuid <new credential.vless_uuid>
next yaoe apply
next yaoe publish config
```

`yaoe rotate reality-keypair` updates exactly `credential.reality_private_key` and `credential.reality_short_id`. It invokes `sing-box generate reality-keypair`, stores only the private key, derives the public key locally, and requires the derived public key to equal the generated public key. It prints:

```text
reality_private_key <new credential.reality_private_key>
reality_public_key <derived Reality public key>
reality_short_id <new credential.reality_short_id>
next yaoe apply
next yaoe publish config
```

Rotate commands preserve unrelated TOML content and comments by editing with a TOML-preserving editor.

### 4.4 Runtime Binary Cache

For managed server installation, the controller requires the matching cached executable after resolving the selected server runtime variant:

```text
.yaoe/cache/server-runtime/sing-box/1.13.13/linux-amd64/sing-box
.yaoe/cache/server-runtime/sing-box/1.13.13/linux-arm64/sing-box
```

Server runtime resolution order is exact:

1. Use the cached executable when it exists, is executable, and reports `1.13.13`.
2. Extract the executable from `.yaoe/cache/upstream/sing-box/1.13.13/<runtime-variant>/<runtime-asset>` when that file exists and has at least one byte, then require the extracted executable to report `1.13.13`.
3. Download `<runtime-asset>` from the configured Gitee Release URL, extract it, and require the extracted executable to report `1.13.13`.
4. Download the official upstream sing-box `v1.13.13` Linux release asset for the runtime variant, extract it, and require the extracted executable to report `1.13.13`.
5. Fail with exit code `6` when no valid executable is available after steps 1 through 4.

Every successful resolution writes `.yaoe/cache/server-runtime/sing-box/1.13.13/<runtime-variant>/sing-box` atomically with mode `0755`.

### 4.5 Cache Reuse Policy

Upstream downloads:

```text
if the expected local upstream cache file exists and has at least one byte -> reuse it
if the cache is bad -> operator deletes the cache file and reruns the command
```

Gitee Release uploads:

```text
if .yaoe/cache/published/gitee-release/yaoe-v0.0.1-sing-box-1.13.13/<asset-name>.ok exists -> skip uploading that asset without remote lookup
if the .ok marker is absent and the remote release already has an asset with the same name -> write the .ok marker and skip upload
if the .ok marker is absent and the remote release does not have an asset with the same name -> upload the asset and write the .ok marker
if the remote asset is wrong -> operator deletes the remote asset and the local .ok file, then reruns the command
```

Gitee repository raw file uploads:

```text
if the repository or branch was just created -> upload all four scripts and refresh .last files
else if rendered bytes equal .yaoe/cache/published/gitee-repo/main/<path>.last -> skip uploading that repository file without remote lookup
else if the remote file exists and its bytes equal rendered bytes -> write the .last file and skip upload
else publish the rendered file and atomically replace the .last file
```

R2 config publication:

```text
always PUT all seven config objects
```

## 5. Command Surface

### 5.1 Commands

YAOE commands are exactly:

```text
yaoe init
yaoe check
yaoe client
yaoe rotate config-key
yaoe rotate vless-uuid
yaoe rotate reality-keypair
yaoe apply [<server>]
yaoe publish bootstrap
yaoe publish runtime
yaoe publish config
yaoe publish delivery
yaoe status [<server>]
yaoe health [<server>]
```

### 5.2 Command Roles

| Command                       | Role                                                                                                                                                               | External contact                                          | Writes `.yaoe/` |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- | --------------- |
| `yaoe init`                   | Create fixed local layout and sample config when absent.                                                                                                           | No                                                        | Yes             |
| `yaoe check`                  | Validate local config, home layout, credential syntax, Reality field syntax, derived Reality public key, R2 coordinates, Gitee coordinates, and server uniqueness. | No                                                        | No              |
| `yaoe client`                 | Print Clash Verge Rev profile URLs, mobile Remote Profile URLs, Linux service install/update commands, and macOS service install/update commands.                  | No                                                        | No              |
| `yaoe rotate config-key`      | Regenerate only the config path key.                                                                                                                               | No                                                        | Yes             |
| `yaoe rotate vless-uuid`      | Regenerate only the VLESS UUID.                                                                                                                                    | No                                                        | Yes             |
| `yaoe rotate reality-keypair` | Regenerate Reality private key and short ID.                                                                                                                       | No                                                        | Yes             |
| `yaoe apply`                  | Package every configured server, upload over root SSH, install systemd service, and verify active service state.                                                   | Gitee/GitHub during runtime resolution, SSH               | Yes             |
| `yaoe apply <server>`         | Run the same apply workflow for one named server.                                                                                                                  | Gitee/GitHub during runtime resolution, SSH               | Yes             |
| `yaoe publish bootstrap`      | Publish public Linux/macOS service scripts to Gitee repository branch `main`.                                                                                      | Gitee                                                     | Yes             |
| `yaoe publish runtime`        | Sync upstream sing-box artifacts and SRS files into Gitee Release with cache reuse.                                                                                | GitHub, Gitee                                             | Yes             |
| `yaoe publish config`         | Render, validate, upload, publicly fetch, revalidate, and print seven client config URLs.                                                                          | Cloudflare R2, public HTTPS GET                           | Yes             |
| `yaoe publish delivery`       | Run `publish bootstrap`, `publish runtime`, and `publish config` in that order.                                                                                    | GitHub, Gitee, Cloudflare R2, public HTTPS GET            | Yes             |
| `yaoe status [<server>]`      | Read remote systemd state, remote sing-box version, remote config validity, and remote listen state.                                                               | SSH                                                       | No              |
| `yaoe health [<server>]`      | Run `status` checks and local active sing-box `mixed` inbound probe through each selected Reality outbound.                                                        | SSH, local sing-box, proxied HTTPS through managed server | Yes             |

### 5.3 Command Semantics

`init` creates the fixed `.yaoe/` layout and sample config when the config is absent.

`check` validates local config, home layout, credential syntax, Reality field syntax, derived Reality public key, R2 coordinates, Gitee coordinates, server uniqueness, direct CIDRs, and fixed command-derived paths. It contacts no network service.

`client` derives and prints all client entrypoints. The command uses the validation profile in section 3.5. It does not check whether Gitee scripts or R2 config objects have been published.

`apply` installs every configured server. `apply <server>` installs only the named server. Both forms use the same workflow and validation path.

`publish bootstrap` performs the bootstrap publication sequence in section 6.6.

`publish runtime` performs the runtime publication sequence in section 6.5.

`publish config` performs the R2 config publication sequence in section 6.8.

`publish delivery` performs `publish bootstrap`, `publish runtime`, and `publish config` in that order. When any subcommand-equivalent step fails, later steps are not attempted.

`status [<server>]` runs the remote checks in section 10.1. When `<server>` is absent, it runs against all configured servers in server-name ascending order.

`health [<server>]` runs the remote checks in section 10.1 plus the local active probe in section 10.2. When `<server>` is absent, it runs against all configured servers in server-name ascending order.

### 5.4 `yaoe client` Output Contract

`yaoe client` stdout is exact after placeholder substitution. Blocks appear in this order: `clash-verge remote-profile`, `clash-verge import-url`, `ios remote-profile`, `android remote-profile`, `linux sing-box install`, `linux sing-box update`, `macos sing-box install`, `macos sing-box update`. Blocks are separated by one blank line. The output ends with one newline.

```text
clash-verge remote-profile
https://<cloudflare.delivery_domain>/config/<credential.config_key>/clash-verge.yaml

clash-verge import-url
clash://install-config?url=<percent-encoded-gui-profile-url>

ios remote-profile
https://<cloudflare.delivery_domain>/config/<credential.config_key>/ios.json

android remote-profile
https://<cloudflare.delivery_domain>/config/<credential.config_key>/android.json

linux sing-box install
export YAOE_CONFIG_KEY='<credential.config_key>'
curl -fsSL https://gitee.com/<gitee.owner>/<gitee.repo>/raw/main/install/linux.sh \
  | sudo env YAOE_CONFIG_KEY="$YAOE_CONFIG_KEY" bash

linux sing-box update
export YAOE_CONFIG_KEY='<credential.config_key>'
curl -fsSL https://gitee.com/<gitee.owner>/<gitee.repo>/raw/main/update/linux.sh \
  | sudo env YAOE_CONFIG_KEY="$YAOE_CONFIG_KEY" bash

macos sing-box install
export YAOE_CONFIG_KEY='<credential.config_key>'
curl -fsSL https://gitee.com/<gitee.owner>/<gitee.repo>/raw/main/install/macos.sh \
  | sudo env YAOE_CONFIG_KEY="$YAOE_CONFIG_KEY" /bin/bash

macos sing-box update
export YAOE_CONFIG_KEY='<credential.config_key>'
curl -fsSL https://gitee.com/<gitee.owner>/<gitee.repo>/raw/main/update/macos.sh \
  | sudo env YAOE_CONFIG_KEY="$YAOE_CONFIG_KEY" /bin/bash
```

### 5.5 Exit Codes

| Code | Meaning                                                                                                                         |
| ---- | ------------------------------------------------------------------------------------------------------------------------------- |
| `0`  | Success.                                                                                                                        |
| `2`  | Invalid CLI arguments.                                                                                                          |
| `3`  | Configuration parse, validation, normalization, credential rotation, Reality key derivation, or named-server selection error.   |
| `4`  | Local home, filesystem, cache, work file, generated artifact, or derived state error.                                           |
| `5`  | Local sing-box or mihomo command failed.                                                                                        |
| `6`  | Required server runtime cache entry missing or invalid after resolution.                                                        |
| `8`  | SSH, SCP, or remote command failure.                                                                                            |
| `9`  | Remote installer, systemd validation, remote status validation, or remote health prerequisite failure.                          |
| `10` | Cloudflare zone resolution, R2 bucket, custom-domain, object upload, public config fetch, Wrangler, or delivery-domain failure. |
| `11` | CN direct SRS upstream fetch failure.                                                                                           |
| `12` | Runtime upstream artifact fetch failure.                                                                                        |
| `13` | Gitee repository, branch, or Gitee Release publication failure.                                                                 |
| `14` | Internal invariant violation.                                                                                                   |
| `15` | Local active health probe failure.                                                                                              |

## 6. Delivery Artifacts, Upstream Sync, Gitee Publication, and R2 Publication

### 6.1 Gitee Repository and Release Roles

Gitee Release is the authoritative public store for versioned binary and sing-box rule-set artifacts. Gitee repository raw files are the authoritative public store for Linux/macOS service bootstrap scripts.

Gitee Release assets are exactly:

```text
sing-box-1.13.13-linux-amd64.tar.gz
sing-box-1.13.13-linux-arm64.tar.gz
sing-box-1.13.13-macos-amd64.tar.gz
sing-box-1.13.13-macos-arm64.tar.gz
cn-domain.srs
cn-ipv4.srs
```

Gitee repository raw files published by YAOE are exactly:

```text
install/linux.sh
update/linux.sh
install/macos.sh
update/macos.sh
```

### 6.2 Upstream Artifact Sources

The sing-box upstream source is the SagerNet GitHub Release identified by:

```text
SING_BOX_RELEASE_TAG = "v1.13.13"
```

The implementation maps upstream sing-box release assets to public Gitee assets as follows:

| Variant       | Upstream URL                                                                                           | Public Gitee asset                    |
| ------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------- |
| `linux-amd64` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-linux-amd64.tar.gz`  | `sing-box-1.13.13-linux-amd64.tar.gz` |
| `linux-arm64` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-linux-arm64.tar.gz`  | `sing-box-1.13.13-linux-arm64.tar.gz` |
| `macos-amd64` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-darwin-amd64.tar.gz` | `sing-box-1.13.13-macos-amd64.tar.gz` |
| `macos-arm64` | `https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-darwin-arm64.tar.gz` | `sing-box-1.13.13-macos-arm64.tar.gz` |

The CN direct SRS upstream-to-public mapping is fixed:

| Public tag  | Upstream URL                                                                                           | Local cache path                         | Gitee asset     | Route meaning                                             |
| ----------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------- | --------------- | --------------------------------------------------------- |
| `cn-domain` | `https://cdn.jsdelivr.net/gh/Dreista/sing-box-rule-set-cn@rule-set/accelerated-domains.china.conf.srs` | `.yaoe/cache/upstream/srs/cn-domain.srs` | `cn-domain.srs` | Domains eligible for direct CN DNS and direct CN routing. |
| `cn-ipv4`   | `https://cdn.jsdelivr.net/gh/Dreista/sing-box-rule-set-cn@rule-set/chnroutes.txt.srs`                  | `.yaoe/cache/upstream/srs/cn-ipv4.srs`   | `cn-ipv4.srs`   | IPv4 destinations eligible for direct CN routing.         |

A CN direct SRS fetch succeeds when the HTTPS GET response status is `200`, the response body has at least one byte, the bytes are atomically written to the expected local cache path, and a generated validation config that references the local binary rule set passes `sing-box check -c` with sing-box `1.13.13`.

### 6.3 Public URL Construction

For every configured Gitee coordinate, YAOE constructs public raw URLs in this exact form:

```text
https://gitee.com/<gitee.owner>/<gitee.repo>/raw/main/<path>
```

YAOE constructs release asset URLs in this exact form:

```text
https://gitee.com/<gitee.owner>/<gitee.repo>/releases/download/yaoe-v0.0.1-sing-box-1.13.13/<asset-name>
```

### 6.4 Gitee API and Git Operations

All Gitee API calls use HTTPS under `https://gitee.com/api/v5`. The controller passes `access_token=<gitee.token>` as an HTTPS query parameter on every Gitee API request. Logs and local files redact the token.

Required Gitee API endpoints:

| Purpose                        | Endpoint                                                        |
| ------------------------------ | --------------------------------------------------------------- |
| Read authenticated user        | `GET /user`                                                     |
| Read repository                | `GET /repos/{owner}/{repo}`                                     |
| Create personal repository     | `POST /user/repos`                                              |
| Create organization repository | `POST /orgs/{org}/repos`                                        |
| Read release by tag            | `GET /repos/{owner}/{repo}/releases/tags/{tag}`                 |
| Create release                 | `POST /repos/{owner}/{repo}/releases`                           |
| List release attachments       | `GET /repos/{owner}/{repo}/releases/{release_id}/attach_files`  |
| Upload release attachment      | `POST /repos/{owner}/{repo}/releases/{release_id}/attach_files` |

Git operations use remote URL `https://gitee.com/<gitee.owner>/<gitee.repo>.git`. Git authentication uses a generated askpass script at `.yaoe/work/gitee-askpass/<nonce>/askpass.sh` with mode `0700`. The script returns the authenticated user's login for prompts containing `Username` and `gitee.token` for prompts containing `Password`. Every `git fetch`, `git commit`, and `git push` invocation runs with `GIT_TERMINAL_PROMPT=0`, `GIT_ASKPASS=<askpass-path>`, and `-c credential.helper=`. The controller deletes `.yaoe/work/gitee-askpass/<nonce>/` before returning success or failure.

### 6.5 `publish runtime` Sequence

`yaoe publish runtime` performs this exact sequence:

1. Load and validate `.yaoe/yaoe.toml`.
2. Ensure the Gitee delivery repository exists.
3. Ensure branch `main` exists.
4. Ensure the fixed Gitee Release exists.
5. For each sing-box artifact in section 6.2 registry order, reuse the corresponding local upstream cache file when it exists and has at least one byte.
6. For each missing sing-box cache file, fetch the mapped upstream URL and atomically write the mapped public-cache file name.
7. Reuse `.yaoe/cache/upstream/srs/cn-domain.srs` when it exists, has at least one byte, and passes local binary rule-set validation.
8. Otherwise fetch `CN_DOMAIN_UPSTREAM_URL`, atomically write `.yaoe/cache/upstream/srs/cn-domain.srs`, and require local binary rule-set validation to pass.
9. Reuse `.yaoe/cache/upstream/srs/cn-ipv4.srs` when it exists, has at least one byte, and passes local binary rule-set validation.
10. Otherwise fetch `CN_IPV4_UPSTREAM_URL`, atomically write `.yaoe/cache/upstream/srs/cn-ipv4.srs`, and require local binary rule-set validation to pass.
11. For each asset in section 6.1 order, apply the release asset publication sequence.

### 6.6 `publish bootstrap` Sequence

`yaoe publish bootstrap` performs this exact sequence:

1. Load and validate `.yaoe/yaoe.toml`.
2. Ensure the Gitee delivery repository exists.
3. Render four service scripts to `.yaoe/work/delivery/gitee-repo/install/` and `.yaoe/work/delivery/gitee-repo/update/`.
4. Ensure branch `main` exists.
5. For each repository raw file in section 6.1, compare rendered bytes to `.yaoe/cache/published/gitee-repo/main/<path>.last` when the file exists and the repository and branch were not just created.
6. Skip remote lookup and upload for a file when the local `.last` bytes are identical to rendered bytes.
7. For every remaining file, compare rendered bytes to the current remote branch bytes from fetched `origin/main` when the remote file exists.
8. When remote bytes equal rendered bytes, atomically replace the corresponding `.last` file with the rendered bytes and skip upload.
9. Commit and push every changed rendered file to branch `main` with message `yaoe v0.0.1 service scripts`.
10. After a successful push, atomically replace the corresponding `.last` files with the rendered bytes.

### 6.7 R2 Delivery Surface

The config delivery surface is exactly one Cloudflare R2 bucket connected to exactly one custom domain:

```text
cloudflare.r2_bucket
cloudflare.delivery_domain
```

The deployed public config URLs are exactly:

```text
GET https://<cloudflare.delivery_domain>/config/<credential.config_key>/clash-verge.yaml
GET https://<cloudflare.delivery_domain>/config/<credential.config_key>/linux-amd64.json
GET https://<cloudflare.delivery_domain>/config/<credential.config_key>/linux-arm64.json
GET https://<cloudflare.delivery_domain>/config/<credential.config_key>/macos-amd64.json
GET https://<cloudflare.delivery_domain>/config/<credential.config_key>/macos-arm64.json
GET https://<cloudflare.delivery_domain>/config/<credential.config_key>/ios.json
GET https://<cloudflare.delivery_domain>/config/<credential.config_key>/android.json
```

Each JSON object is uploaded with exactly these HTTP headers:

```text
Content-Type: application/json; charset=utf-8
Cache-Control: no-store
```

The YAML object is uploaded with exactly these HTTP headers:

```text
Content-Type: text/yaml; charset=utf-8
Cache-Control: no-store
```

### 6.8 `publish config` Sequence

`yaoe publish config` performs this exact sequence:

1. Load and validate `.yaoe/yaoe.toml` using full validation.
2. Require `sing-box` from `PATH` to report version `1.13.13`.
3. Require `mihomo` from `PATH` to report version `1.19.27`.
4. Resolve the Cloudflare zone that owns `cloudflare.delivery_domain`.
5. Invoke `wrangler r2 bucket list` and determine whether `cloudflare.r2_bucket` exists.
6. Invoke `wrangler r2 bucket create <cloudflare.r2_bucket>` when the bucket is absent.
7. Invoke `wrangler r2 bucket domain get <cloudflare.r2_bucket> --domain <cloudflare.delivery_domain>`.
8. Invoke `wrangler r2 bucket domain add <cloudflare.r2_bucket> --domain <cloudflare.delivery_domain> --zone-id <derived_cloudflare_zone_id> --min-tls 1.2 --force` when the custom domain is absent.
9. Invoke `wrangler r2 bucket domain update <cloudflare.r2_bucket> --domain <cloudflare.delivery_domain> --min-tls 1.2` when the domain exists with a different minimum TLS version.
10. Render platform configs to `.yaoe/work/delivery/rendered-config/` for exactly the seven variants in `CONFIG_VARIANTS`.
11. Run `mihomo -t -f .yaoe/work/delivery/rendered-config/clash-verge.yaml`.
12. Run `sing-box check -c` against the six rendered sing-box JSON configs.
13. Upload all seven config objects through `wrangler r2 object put` with the headers in section 6.7 and `--force`.
14. HTTP GET each public config URL.
15. Repeat the GET for at most `CLOUDFLARE_PUBLIC_FETCH_ATTEMPTS` attempts separated by `CLOUDFLARE_PUBLIC_FETCH_INTERVAL_SECONDS` seconds until every response status is `200`.
16. Write each returned config to a temporary local file.
17. Run `mihomo -t -f` against the returned `clash-verge.yaml`.
18. Run `sing-box check -c` against each returned sing-box JSON config.
19. Print success lines in the exact format `config <variant> <full-config-url>` for every variant in `CONFIG_VARIANTS` order.

## 7. Managed Server Installation

### 7.1 Server Config Semantics

For each server, the controller renders one sing-box config with:

1. One VLESS inbound tagged `vless-in`.
2. Listen address `0.0.0.0`.
3. Listen port from `server.<name>.port`.
4. One VLESS `users` array entry using `credential.vless_uuid`.
5. Vision flow `xtls-rprx-vision`.
6. TLS enabled with Reality enabled.
7. TLS top-level `server_name` from `reality.handshake_server`.
8. Reality handshake server from `reality.handshake_server`.
9. Reality handshake port from `reality.handshake_port` or default `443`.
10. Reality private key from `credential.reality_private_key`.
11. Reality short ID array containing exactly `credential.reality_short_id`.
12. One direct outbound tagged `direct`.
13. Route final `direct`.
14. Log file `/var/log/yaoe/<server>.log`.

The server config top-level object order is `log`, `inbounds`, `outbounds`, `route`.

### 7.2 Server Package and Target Layout

Package staging path:

```text
.yaoe/work/packages/<server>/yaoe-server-package/
```

Package archive path:

```text
.yaoe/work/packages/yaoe-server-<server>.tar.gz
```

Package contents:

```text
yaoe-server-package/
├── install.sh
└── payload/
    ├── bin/sing-box
    ├── config/<server>.json
    └── systemd/yaoe-<server>.service
```

Target layout:

```text
/etc/yaoe/config/<server>.json
/var/lib/yaoe/bin/sing-box
/var/log/yaoe/<server>.log
/etc/systemd/system/yaoe-<server>.service
```

Systemd unit:

```ini
[Unit]
Description=YAOE managed Reality egress server <server>
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root
ExecStart=/var/lib/yaoe/bin/sing-box run -c /etc/yaoe/config/<server>.json
Restart=always
RestartSec=5
WorkingDirectory=/var/lib/yaoe
LimitNOFILE=1048576

[Install]
WantedBy=multi-user.target
```

### 7.3 Target Installer

The embedded `install.sh` performs this exact sequence:

1. Confirms `uname -s` equals `Linux`.
2. Confirms observed architecture matches the package runtime variant: `x86_64` or `amd64` for `linux-amd64`; `aarch64` or `arm64` for `linux-arm64`.
3. Confirms effective uid `0`.
4. Stops existing `yaoe-<server>.service` when it exists.
5. Creates target directories.
6. Writes payload files.
7. Ensures `/var/lib/yaoe/bin/sing-box` is executable with mode `0755`.
8. Requires `/var/lib/yaoe/bin/sing-box version` to report `1.13.13`.
9. Runs `/var/lib/yaoe/bin/sing-box check -c /etc/yaoe/config/<server>.json`.
10. Installs or updates the systemd unit.
11. Runs `systemctl daemon-reload`.
12. Enables and starts the service.
13. Requires `systemctl is-active yaoe-<server>.service` output `active`.

### 7.4 SSH Flow

`yaoe apply` uses root SSH. For each selected server, the controller performs this exact sequence:

1. Resolve the SSH key path.
2. Invoke `ssh` and `scp` with `BatchMode=yes` and `IdentitiesOnly=yes`.
3. Run `uname -s` and require output `Linux`.
4. Run `uname -m` and map `x86_64` or `amd64` to `linux-amd64`, and `aarch64` or `arm64` to `linux-arm64`.
5. Resolve the matching server runtime.
6. Render the server config and assemble the package with the matching runtime.
7. Upload the package to a unique `/tmp/` path.
8. Extract the package in a unique `/tmp/` directory.
9. Run `install.sh` as root.
10. Remove uploaded and extracted files after installer success.
11. Require remote `systemctl is-active yaoe-<server>.service` output `active`.

## 8. Client Delivery

### 8.1 Public Entrypoints

The only command that prints client-facing entrypoints is:

```bash
yaoe client
```

The command prints:

1. One Clash Verge Rev remote-profile URL.
2. One Clash Verge Rev URL Scheme import URL.
3. Two mobile sing-box Remote Profile URLs.
4. Four Linux/macOS sing-box service script commands.

The only environment variable consumed by Linux/macOS service scripts is:

```text
YAOE_CONFIG_KEY
```

`YAOE_CONFIG_KEY` matches:

```text
^[A-Za-z0-9_-]{128}$
```

### 8.2 Clash Verge Rev GUI Profile Delivery

Desktop GUI users install Clash Verge Rev and import one generated profile:

```text
https://<cloudflare.delivery_domain>/config/<credential.config_key>/clash-verge.yaml
```

The URL Scheme form is:

```text
clash://install-config?url=<percent-encoded-gui-profile-url>
```

The `clash-verge.yaml` profile is complete. It contains all nodes, proxy groups, DNS, TUN, geodata URLs, and rules. Users do not supply node fields, rules, providers, JavaScript extension scripts, merge files, or subscription conversion settings.

### 8.3 Mobile Public Entrypoints

The iOS config URL is the `ios remote-profile` URL printed by `yaoe client`.

The Android config URL is the `android remote-profile` URL printed by `yaoe client`.

Each URL is a complete unauthenticated HTTPS URL. The operator imports that URL into the official sing-box graphical client as a Remote Profile.

### 8.4 Linux Install Script

Linux local artifact paths:

```text
/usr/local/libexec/yaoe/sing-box
/etc/yaoe-sing-box/config.json
/etc/systemd/system/yaoe-sing-box.service
```

The Linux install script performs this exact sequence:

1. Requires `uname -s` to be `Linux`.
2. Maps `uname -m` of `x86_64` or `amd64` to `arch=amd64`.
3. Maps `uname -m` of `aarch64` or `arm64` to `arch=arm64`.
4. Rejects every other `uname -m` value.
5. Derives `variant=linux-$arch`.
6. Requires effective uid `0`.
7. Requires `YAOE_CONFIG_KEY` to match section 8.1.
8. Creates `/usr/local/libexec/yaoe` with mode `0755`.
9. Creates `/etc/yaoe-sing-box` with mode `0755`.
10. Downloads `sing-box-1.13.13-linux-$arch.tar.gz` from the Gitee Release asset URL embedded in the rendered script.
11. Extracts the sing-box executable to a temporary directory.
12. Installs the executable to `/usr/local/libexec/yaoe/sing-box` with mode `0755`.
13. Requires `/usr/local/libexec/yaoe/sing-box version` to report `1.13.13`.
14. Constructs `https://<cloudflare.delivery_domain>/config/$YAOE_CONFIG_KEY/$variant.json`.
15. Downloads the config to `/etc/yaoe-sing-box/config.json.pending`.
16. Runs `/usr/local/libexec/yaoe/sing-box check -c /etc/yaoe-sing-box/config.json.pending`.
17. Renders `/etc/systemd/system/yaoe-sing-box.service`.
18. Atomically replaces `/etc/yaoe-sing-box/config.json` with the pending config.
19. Runs `systemctl daemon-reload`.
20. Runs `systemctl enable --now yaoe-sing-box.service`.
21. Runs `systemctl restart yaoe-sing-box.service`.
22. Requires `systemctl is-active yaoe-sing-box.service` output `active`.

Linux systemd unit:

```ini
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
```

### 8.5 Linux Update Script

The Linux update script performs this exact sequence:

1. Requires `uname -s` to be `Linux`.
2. Maps CPU architecture exactly as section 8.4.
3. Derives `variant=linux-$arch`.
4. Requires effective uid `0`.
5. Requires `YAOE_CONFIG_KEY` to match section 8.1.
6. Requires `/usr/local/libexec/yaoe/sing-box` to exist and be executable.
7. Requires `/etc/systemd/system/yaoe-sing-box.service` to exist.
8. Requires `/usr/local/libexec/yaoe/sing-box version` to report `1.13.13`.
9. Creates `/etc/yaoe-sing-box` with mode `0755` when the directory is absent.
10. Constructs `https://<cloudflare.delivery_domain>/config/$YAOE_CONFIG_KEY/$variant.json`.
11. Downloads the config to `/etc/yaoe-sing-box/config.json.pending`.
12. Runs `/usr/local/libexec/yaoe/sing-box check -c /etc/yaoe-sing-box/config.json.pending`.
13. Atomically replaces `/etc/yaoe-sing-box/config.json` with the pending config.
14. Runs `systemctl restart yaoe-sing-box.service`.
15. Requires `systemctl is-active yaoe-sing-box.service` output `active`.

### 8.6 macOS Install Script

macOS local artifact paths:

```text
/usr/local/libexec/yaoe/sing-box
/Library/Application Support/YAOE/sing-box/config.json
/Library/LaunchDaemons/io.yaoe.sing-box.plist
```

The macOS install script performs this exact sequence:

1. Requires `uname -s` to be `Darwin`.
2. Maps `uname -m` of `x86_64` to `arch=amd64`.
3. Maps `uname -m` of `arm64` to `arch=arm64`.
4. Rejects every other `uname -m` value.
5. Derives `variant=macos-$arch`.
6. Requires effective uid `0`.
7. Requires `YAOE_CONFIG_KEY` to match section 8.1.
8. Creates `/usr/local/libexec/yaoe` with mode `0755`.
9. Creates `/Library/Application Support/YAOE/sing-box` with mode `0755`.
10. Downloads `sing-box-1.13.13-macos-$arch.tar.gz` from the Gitee Release asset URL embedded in the rendered script.
11. Extracts the sing-box executable to a temporary directory.
12. Installs the executable to `/usr/local/libexec/yaoe/sing-box` with mode `0755`.
13. Requires `/usr/local/libexec/yaoe/sing-box version` to report `1.13.13`.
14. Constructs `https://<cloudflare.delivery_domain>/config/$YAOE_CONFIG_KEY/$variant.json`.
15. Downloads the config to `/Library/Application Support/YAOE/sing-box/config.json.pending`.
16. Runs `/usr/local/libexec/yaoe/sing-box check -c '/Library/Application Support/YAOE/sing-box/config.json.pending'`.
17. Renders `/Library/LaunchDaemons/io.yaoe.sing-box.plist` with owner `root:wheel` and mode `0644`.
18. Atomically replaces `/Library/Application Support/YAOE/sing-box/config.json` with the pending config.
19. Runs `launchctl bootout system /Library/LaunchDaemons/io.yaoe.sing-box.plist` when the service is loaded.
20. Runs `launchctl bootstrap system /Library/LaunchDaemons/io.yaoe.sing-box.plist`.
21. Runs `launchctl enable system/io.yaoe.sing-box`.
22. Requires `launchctl print system/io.yaoe.sing-box` to succeed.

Launchd plist:

```xml
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
```

### 8.7 macOS Update Script

The macOS update script performs this exact sequence:

1. Requires `uname -s` to be `Darwin`.
2. Detects CPU architecture exactly as section 8.6.
3. Derives `variant=macos-$arch`.
4. Requires effective uid `0`.
5. Requires `YAOE_CONFIG_KEY` to match section 8.1.
6. Requires `/usr/local/libexec/yaoe/sing-box` to exist and be executable.
7. Requires `/Library/LaunchDaemons/io.yaoe.sing-box.plist` to exist.
8. Requires `/usr/local/libexec/yaoe/sing-box version` to report `1.13.13`.
9. Creates `/Library/Application Support/YAOE/sing-box` with mode `0755` when the directory is absent.
10. Constructs `https://<cloudflare.delivery_domain>/config/$YAOE_CONFIG_KEY/$variant.json`.
11. Downloads the config to `/Library/Application Support/YAOE/sing-box/config.json.pending`.
12. Runs `/usr/local/libexec/yaoe/sing-box check -c '/Library/Application Support/YAOE/sing-box/config.json.pending'`.
13. Atomically replaces `/Library/Application Support/YAOE/sing-box/config.json` with the pending config.
14. Runs `launchctl kickstart -k system/io.yaoe.sing-box`.
15. Requires `launchctl print system/io.yaoe.sing-box` to succeed.

## 9. Generated Client Configs

### 9.1 Generation Model

Platform client configs are generated by `yaoe publish config` and by the config sub-step inside `yaoe publish delivery`.

The rendered platform configs are generated work files at:

```text
.yaoe/work/delivery/rendered-config/clash-verge.yaml
.yaoe/work/delivery/rendered-config/linux-amd64.json
.yaoe/work/delivery/rendered-config/linux-arm64.json
.yaoe/work/delivery/rendered-config/macos-amd64.json
.yaoe/work/delivery/rendered-config/macos-arm64.json
.yaoe/work/delivery/rendered-config/ios.json
.yaoe/work/delivery/rendered-config/android.json
```

These files are overwritten during `yaoe publish config` and `yaoe publish delivery`.

### 9.2 Clash Verge Rev mihomo Profile Semantics

The `clash-verge.yaml` profile is valid mihomo YAML. The renderer emits YAML with two-space indentation, stable key order, and trailing newline.

The top-level profile contains exactly these major sections in order:

```text
mixed-port
allow-lan
mode
log-level
ipv6
geodata-mode
geo-auto-update
geo-update-interval
geox-url
dns
tun
proxies
proxy-groups
rules
```

Top-level scalar values are exact:

```yaml
mixed-port: 7890
allow-lan: false
mode: rule
log-level: info
ipv6: false
geodata-mode: true
geo-auto-update: true
geo-update-interval: 24
```

`geox-url` is exact:

```yaml
geox-url:
  geoip: https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geoip.dat
  geosite: https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/geosite.dat
  mmdb: https://testingcf.jsdelivr.net/gh/MetaCubeX/meta-rules-dat@release/country.mmdb
```

DNS is exact except YAML ordering is the order shown:

```yaml
dns:
  enable: true
  ipv6: false
  enhanced-mode: fake-ip
  fake-ip-range: 198.18.0.1/16
  default-nameserver:
    - 223.5.5.5
    - 119.29.29.29
  nameserver:
    - https://dns.alidns.com/dns-query
    - https://doh.pub/dns-query
  fallback:
    - tls://1.1.1.1
    - tls://8.8.8.8
  fallback-filter:
    geoip: true
    geoip-code: CN
```

TUN is exact:

```yaml
tun:
  enable: true
  stack: mixed
  auto-route: true
  auto-detect-interface: true
  strict-route: true
  dns-hijack:
    - any:53
    - tcp://any:53
```

For each configured server in server-name ascending order, `proxies` contains one VLESS Reality node with this shape:

```yaml
proxies:
  - name: egress-<server-name>
    type: vless
    server: <server.<name>.ip>
    port: <server.<name>.port>
    uuid: <credential.vless_uuid>
    network: tcp
    tls: true
    udp: true
    flow: xtls-rprx-vision
    servername: <reality.handshake_server>
    reality-opts:
      public-key: <derived_reality_public_key>
      short-id: <credential.reality_short_id>
      support-x25519mlkem768: false
    client-fingerprint: chrome
```

The proxy group is exact except the proxy list follows server-name ascending order:

```yaml
proxy-groups:
  - name: PROXY
    type: url-test
    proxies:
      - egress-<server-name>
    url: https://www.gstatic.com/generate_204
    interval: 300
```

The rules list is exact in this order:

```yaml
rules:
  - IP-CIDR,127.0.0.0/8,DIRECT,no-resolve
  - IP-CIDR,169.254.0.0/16,DIRECT,no-resolve
  - IP-CIDR,224.0.0.0/4,DIRECT,no-resolve
  - IP-CIDR,10.0.0.0/8,DIRECT,no-resolve
  - IP-CIDR,172.16.0.0/12,DIRECT,no-resolve
  - IP-CIDR,192.168.0.0/16,DIRECT,no-resolve
  - IP-CIDR,<configured direct CIDR>,DIRECT,no-resolve
  - IP-CIDR,<server endpoint IPv4>/32,DIRECT,no-resolve
  - GEOSITE,cn,DIRECT
  - GEOIP,CN,DIRECT
  - MATCH,PROXY
```

Direct CIDR construction order is built-in IPv4 CIDRs, configured IPv4 direct CIDRs, and managed-server endpoint IPv4 `/32` CIDRs. Canonical duplicates introduced by merging these lists are skipped when the same canonical CIDR already exists earlier in the generated list.

### 9.3 sing-box Service and Mobile Shared Config Semantics

Every generated sing-box service or mobile config contains:

1. `log.level = "info"`.
2. One TUN inbound tagged `tun-in`.
3. TUN `address = ["172.19.0.1/30", "fdfe:dcba:9876::1/126"]`.
4. TUN `mtu = 1500`.
5. TUN `auto_route = true`.
6. `route.default_domain_resolver = "remote-dns"`.
7. DNS server `cn-dns` is AliDNS DoT over `direct`: `type = "tls"`, `server = "223.5.5.5"`, `server_port = 853`, `detour = "direct"`, and `tls.server_name = "dns.alidns.com"`.
8. DNS server `remote-dns` is Cloudflare DoT over `proxy`: `type = "tls"`, `server = "1.1.1.1"`, `server_port = 853`, `detour = "proxy"`, and `tls.server_name = "cloudflare-dns.com"`.
9. DNS uses global `strategy = "ipv4_only"`.
10. DNS `reverse_mapping = true`.
11. One VLESS outbound for each configured server.
12. Each VLESS outbound connects to `server.<name>.ip` and `server.<name>.port`.
13. Each VLESS outbound uses `credential.vless_uuid`.
14. Each VLESS outbound sets `flow = "xtls-rprx-vision"`.
15. Each VLESS outbound enables TLS Reality with `server_name = reality.handshake_server`.
16. Each VLESS outbound enables uTLS with `fingerprint = "chrome"`.
17. Each VLESS outbound uses the derived Reality public key and `credential.reality_short_id`.
18. One URLTest outbound tagged `proxy` containing every server outbound tag.
19. One direct outbound tagged `direct`.
20. First route rule `{ "port": 53, "action": "hijack-dns" }`.
21. Second route rule `{ "action": "sniff" }`.
22. Third route rule routes direct CIDRs to `direct` with `action = "route"`.
23. Fourth route rule rejects public IPv6 with `ip_version = 6`, `action = "reject"`, `method = "default"`, and `no_drop = true`.
24. CN direct remote rule sets `cn-domain` and `cn-ipv4`.
25. Remote rule-set URLs are Gitee Release asset URLs.
26. Remote rule sets use `download_detour = "direct"`.
27. Remote rule sets use `update_interval = "1d"`.
28. CN direct route rule for `cn-domain` and `cn-ipv4` appears after the IPv6 rejection rule.
29. Final route `proxy`.

The built-in IPv4 direct CIDRs are exactly:

```text
127.0.0.0/8
169.254.0.0/16
224.0.0.0/4
10.0.0.0/8
172.16.0.0/12
192.168.0.0/16
```

The built-in IPv6 direct CIDRs are exactly:

```text
::1/128
fe80::/10
fc00::/7
ff00::/8
```

Direct CIDR construction order is built-in IPv4 CIDRs, configured IPv4 direct CIDRs, managed-server endpoint IPv4 `/32` CIDRs, then built-in IPv6 CIDRs. Canonical duplicates introduced by merging built-in CIDRs, user-provided CIDRs, egress server IP CIDRs, and built-in IPv6 CIDRs are skipped when the same canonical CIDR already exists earlier in the generated list.

### 9.4 sing-box Platform Profile Semantics

TUN profile `linux-service` contains:

```json
{
  "auto_redirect": true,
  "strict_route": true
}
```

TUN profile `macos-service` contains:

```json
{
  "strict_route": true
}
```

TUN profile `mobile` contains the shared TUN fields only.

Route profile `service` contains:

```json
{
  "auto_detect_interface": true
}
```

Route profile `mobile` contains the shared route fields only.

The effective mapping is:

| Config variant | TUN profile     | Route profile |
| -------------- | --------------- | ------------- |
| `linux-amd64`  | `linux-service` | `service`     |
| `linux-arm64`  | `linux-service` | `service`     |
| `macos-amd64`  | `macos-service` | `service`     |
| `macos-arm64`  | `macos-service` | `service`     |
| `ios`          | `mobile`        | `mobile`      |
| `android`      | `mobile`        | `mobile`      |

### 9.5 Required sing-box Service Config Shape

Every Linux/macOS service config is equivalent to this shape after substitution and profile insertion. The renderer emits object keys in the order shown for stable reviewable output.

```json
{
  "log": { "level": "info" },
  "dns": {
    "servers": [
      {
        "type": "tls",
        "tag": "cn-dns",
        "server": "223.5.5.5",
        "server_port": 853,
        "detour": "direct",
        "tls": { "server_name": "dns.alidns.com" }
      },
      {
        "type": "tls",
        "tag": "remote-dns",
        "server": "1.1.1.1",
        "server_port": 853,
        "detour": "proxy",
        "tls": { "server_name": "cloudflare-dns.com" }
      }
    ],
    "rules": [
      { "rule_set": ["cn-domain"], "action": "route", "server": "cn-dns" }
    ],
    "final": "remote-dns",
    "strategy": "ipv4_only",
    "reverse_mapping": true
  },
  "inbounds": [
    {
      "type": "tun",
      "tag": "tun-in",
      "address": ["172.19.0.1/30", "fdfe:dcba:9876::1/126"],
      "mtu": 1500,
      "auto_route": true
    }
  ],
  "outbounds": [
    {
      "type": "urltest",
      "tag": "proxy",
      "outbounds": ["egress-hk", "egress-jp"]
    },
    {
      "type": "vless",
      "tag": "egress-hk",
      "server": "203.0.113.20",
      "server_port": 28443,
      "uuid": "<credential.vless_uuid>",
      "flow": "xtls-rprx-vision",
      "tls": {
        "enabled": true,
        "server_name": "<reality.handshake_server>",
        "utls": { "enabled": true, "fingerprint": "chrome" },
        "reality": {
          "enabled": true,
          "public_key": "<derived_reality_public_key>",
          "short_id": "<credential.reality_short_id>"
        }
      }
    },
    { "type": "direct", "tag": "direct" }
  ],
  "route": {
    "default_domain_resolver": "remote-dns",
    "rule_set": [
      {
        "type": "remote",
        "tag": "cn-domain",
        "format": "binary",
        "url": "https://gitee.com/<owner>/<repo>/releases/download/yaoe-v0.0.1-sing-box-1.13.13/cn-domain.srs",
        "download_detour": "direct",
        "update_interval": "1d"
      },
      {
        "type": "remote",
        "tag": "cn-ipv4",
        "format": "binary",
        "url": "https://gitee.com/<owner>/<repo>/releases/download/yaoe-v0.0.1-sing-box-1.13.13/cn-ipv4.srs",
        "download_detour": "direct",
        "update_interval": "1d"
      }
    ],
    "rules": [
      { "port": 53, "action": "hijack-dns" },
      { "action": "sniff" },
      {
        "ip_cidr": [
          "127.0.0.0/8",
          "169.254.0.0/16",
          "224.0.0.0/4",
          "10.0.0.0/8",
          "172.16.0.0/12",
          "192.168.0.0/16",
          "100.64.0.0/10",
          "203.0.113.20/32",
          "::1/128",
          "fe80::/10",
          "fc00::/7",
          "ff00::/8"
        ],
        "action": "route",
        "outbound": "direct"
      },
      {
        "ip_version": 6,
        "action": "reject",
        "method": "default",
        "no_drop": true
      },
      {
        "rule_set": ["cn-domain", "cn-ipv4"],
        "action": "route",
        "outbound": "direct"
      }
    ],
    "final": "proxy"
  }
}
```

Profile insertion rules:

1. For `linux-amd64` and `linux-arm64`, insert `auto_redirect = true` and `strict_route = true` into the TUN inbound after `auto_route`, and insert `auto_detect_interface = true` as the first key in `route`.
2. For `macos-amd64` and `macos-arm64`, insert `strict_route = true` into the TUN inbound after `auto_route`, and insert `auto_detect_interface = true` as the first key in `route`.
3. For `ios` and `android`, use mobile profile insertion from section 9.4.

### 9.6 Health Probe Config Shape

The rendered health probe config for a server is equivalent to this shape after substitution. The renderer emits object keys in the order shown.

```json
{
  "log": { "level": "debug" },
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": 0,
      "set_system_proxy": false
    }
  ],
  "outbounds": [
    {
      "type": "vless",
      "tag": "probe",
      "server": "203.0.113.20",
      "server_port": 28443,
      "uuid": "<credential.vless_uuid>",
      "flow": "xtls-rprx-vision",
      "tls": {
        "enabled": true,
        "server_name": "<reality.handshake_server>",
        "utls": { "enabled": true, "fingerprint": "chrome" },
        "reality": {
          "enabled": true,
          "public_key": "<derived_reality_public_key>",
          "short_id": "<credential.reality_short_id>"
        }
      }
    },
    { "type": "direct", "tag": "direct" }
  ],
  "route": { "final": "probe" }
}
```

`listen_port` is replaced by the selected local probe port before the config is written. The probe validates Reality coherence by exercising this tuple:

```text
server.<name>.ip
server.<name>.port
credential.vless_uuid
reality.handshake_server
server TLS server_name = reality.handshake_server
credential.reality_private_key -> derived public key
credential.reality_short_id
flow = xtls-rprx-vision
```

## 10. Status and Health

### 10.1 `status` Remote Checks

`yaoe status [<server>]` performs this exact sequence for each selected server:

1. Load and validate `.yaoe/yaoe.toml` using full validation.
2. Select all servers by server-name ascending order when `<server>` is absent.
3. Reject an unknown `<server>` with exit code `3` before external side effects.
4. Resolve the SSH key path.
5. Invoke `ssh` with `BatchMode=yes` and `IdentitiesOnly=yes`.
6. Run `systemctl is-active yaoe-<server>.service` and require output `active`.
7. Run `systemctl is-enabled yaoe-<server>.service` and record the output.
8. Run `systemctl show yaoe-<server>.service --property=MainPID --value` and require a non-zero integer.
9. Run `/var/lib/yaoe/bin/sing-box version` and require version `1.13.13`.
10. Run `/var/lib/yaoe/bin/sing-box check -c /etc/yaoe/config/<server>.json`.
11. Run a remote TCP listen check and require the configured port to be listening.

The remote TCP listen check command is:

```bash
ss -H -ltn sport = :<server.port>
```

The listen check succeeds when stdout contains at least one line and at least one line contains `0.0.0.0:<port>` or `<server.ip>:<port>`.

`status` stdout prints one summary line per selected server in server-name ascending order:

```text
status <server> active=<active-state> enabled=<enabled-state> pid=<main-pid> version=1.13.13 config=ok listen=<server.ip>:<server.port>
```

### 10.2 `health` Active Probe

`yaoe health [<server>]` performs this exact sequence:

1. Load and validate `.yaoe/yaoe.toml` using full validation.
2. Require local `sing-box` from `PATH` to report version `1.13.13`.
3. Select all servers by server-name ascending order when `<server>` is absent.
4. Reject an unknown `<server>` with exit code `3` before external side effects.
5. Run `status` remote checks from section 10.1 for each selected server.
6. For each selected server, render `.yaoe/work/health/<server>/probe.json`.
7. Reserve one local probe port by binding `127.0.0.1:0`, reading the assigned port, and closing the reservation immediately before starting the probe process.
8. Start `sing-box run -c .yaoe/work/health/<server>/probe.json` as a child process.
9. Wait until a TCP connection to `127.0.0.1:<probe-port>` succeeds or `HEALTH_PROBE_STARTUP_TIMEOUT_SECONDS` elapses.
10. Run `curl` through the SOCKS5 side of the local `mixed` inbound using SOCKS5 remote hostname resolution:

```bash
curl -fsS \
  --ipv4 \
  --socks5-hostname 127.0.0.1:<probe-port> \
  --connect-timeout <HEALTH_PROBE_REQUEST_TIMEOUT_SECONDS> \
  --max-time <HEALTH_PROBE_TOTAL_TIMEOUT_SECONDS> \
  --output /dev/null \
  --write-out '%{http_code}' \
  https://www.gstatic.com/generate_204
```

11. Require curl stdout to equal `204`.
12. Stop the local sing-box child process.

When a probe port bind or sing-box startup fails, the controller retries with a newly selected port until `HEALTH_PROBE_PORT_RETRY_LIMIT` attempts have failed. After the retry limit, the command exits with code `15`.

`health` stdout prints one summary line per selected server in server-name ascending order:

```text
health <server> status=204 url=https://www.gstatic.com/generate_204 via=<server.ip>:<server.port> elapsed_ms=<milliseconds>
```

## 11. Functional Completion and Acceptance

### 11.1 Repository Functional Gate

The implementation is locally complete when these commands pass from the repository root after direnv has loaded the devShell:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo install --path crates/yaoe-cli --locked --force
yaoe --help
```

### 11.2 nextest Configuration

The repository nextest configuration file is:

```text
.config/nextest.toml
```

It contains exactly these profile definitions for `v0.0.1`:

```toml
[profile.default]
fail-fast = true
retries = 0
slow-timeout = "30s"

[profile.acceptance]
inherits = "default"
test-threads = 1
slow-timeout = { period = "120s", terminate-after = 5, grace-period = "5s" }
```

### 11.3 Functional Test Scope

The non-acceptance functional suite validates behavior that changes generated artifacts, command output, external command planning, local state, or runtime reachability. Functional tests cover:

1. `.yaoe/yaoe.toml` parsing, defaults, normalization, validation, placeholder rejection, and derived-value rejection.
2. Credential generation, Reality public key derivation, and rotate-command atomic TOML edits.
3. Platform registry contents: four service variants, two mobile variants, one GUI variant, four service scripts, six Gitee Release assets, and seven R2 config objects.
4. `yaoe client` stdout exact block order and exact URL construction.
5. CN direct SRS fetch, local validation, cache reuse, and Gitee Release publication planning.
6. Generated `clash-verge.yaml` validation with mihomo `1.19.27` and exact coverage of DNS, TUN, geodata, VLESS Reality nodes, URLTest group, and fixed route rule order.
7. Generated sing-box configs validation with sing-box `1.13.13` and exact coverage of DNS hijack, IPv4-only DNS, public IPv6 rejection, CN direct SRS, URLTest, and VLESS Reality outbounds.
8. Linux service script behavior: OS, privilege, config key, CPU detection, runtime download, config download, config check, systemd unit install, service start, service status.
9. macOS service script behavior: OS, privilege, config key, CPU detection, runtime download, config download, config check, launchd plist install, service start, service status.
10. Managed server config rendering, package assembly, target installer checks, and apply workflow.
11. `yaoe status` SSH command construction, systemd checks, version check, config check, listen check, stdout summary, and stderr grammar.
12. `yaoe health` local sing-box version check, remote status prerequisites, health probe config rendering, local mixed inbound startup, curl SOCKS5 remote hostname resolution, HTTP `204` success output, and probe shutdown.
13. Provider-secret redaction and delivery-credential stdout placement.
14. Exit-code mapping by failing product boundary.
15. ANSI color policy.

### 11.4 Real Acceptance Validation

Real acceptance validation is nextest-controlled and validates the complete controller workflow through actual provider surfaces, actual managed server installation, and actual active Reality health probe.

Acceptance test entrypoint:

```text
crates/yaoe-controller/tests/acceptance_delivery.rs
```

Test function name:

```rust
#[test]
#[ignore]
fn acceptance_delivery() { /* implementation */ }
```

Acceptance commands:

```bash
cargo install --path crates/yaoe-cli --locked --force
cargo nextest run -p yaoe-controller -P acceptance --run-ignored=only acceptance_delivery
```

The acceptance test executes this exact flow from the repository root:

```bash
yaoe check
yaoe publish delivery
yaoe apply
yaoe status
yaoe health
yaoe client
```

Acceptance requires `.yaoe/yaoe.toml` to contain real provider credentials, real Gitee coordinates, real Cloudflare R2 coordinates, and at least one real managed Linux amd64 or arm64 server reachable over root SSH. Acceptance records command output only as nextest test output.

## 12. README Contract

The README presents the first-time flow in this order:

1. Load dev environment with `direnv allow`.
2. Install the YAOE CLI with Cargo.
3. Run `yaoe init`.
4. Configure `.yaoe/yaoe.toml` by replacing all placeholders.
5. Ensure the Cloudflare API token can make the R2 bucket, object upload, custom-domain, DNS, and public fetch operations in this document succeed.
6. Ensure the Gitee token can create or update the delivery repository, branch `main`, release, and release attachments.
7. Run `yaoe check`.
8. Run `yaoe publish delivery`.
9. Run `yaoe apply`.
10. Run `yaoe status`.
11. Run `yaoe health`.
12. Print all client entrypoints with:

```bash
yaoe client
```

13. State that Windows users install Clash Verge Rev and import the `clash-verge remote-profile` URL or `clash-verge import-url` printed by `yaoe client`.
14. State that macOS users have two supported entrypoints printed by `yaoe client`: `clash-verge` GUI profile and `macos sing-box` service commands.
15. State that Linux users have two supported entrypoints printed by `yaoe client`: `clash-verge` GUI profile and `linux sing-box` service commands.
16. State that iOS / iPadOS users import the `ios remote-profile` URL from `yaoe client` into the official sing-box graphical client as a Remote Profile.
17. State that Android users import the `android remote-profile` URL from `yaoe client` into the official sing-box graphical client as a Remote Profile.
18. State that Linux and macOS service scripts detect CPU architecture and users pass no architecture parameters.
19. State that generated configs implement IPv4 egress semantics: private/local traffic, configured direct CIDRs, managed-server endpoint IPs, and CN allowlist traffic are direct; remaining public IPv4 traffic uses proxy aggregation.
20. State that Clash Verge Rev users edit no rule files, merge files, script files, or subscription conversion settings.
21. Run real acceptance validation through nextest commands in section 11.4.

The README documents separated publication commands for diagnostics in this order:

```bash
yaoe publish bootstrap
yaoe publish runtime
yaoe publish config
```

The README documents that `yaoe client` is the only supported client-entrypoint derivation command.

The README documents these rotation flows exactly:

Config key rotation:

```bash
yaoe rotate config-key
yaoe publish config
```

VLESS UUID rotation:

```bash
yaoe rotate vless-uuid
yaoe apply
yaoe publish config
```

Reality keypair rotation:

```bash
yaoe rotate reality-keypair
yaoe apply
yaoe publish config
```

## 13. References

### 13.1 Normative Language, Data Formats, and Internet Standards

- [RFC 2119: Key words for use in RFCs to Indicate Requirement Levels](https://www.rfc-editor.org/rfc/rfc2119)
- [RFC 8174: Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words](https://www.rfc-editor.org/rfc/rfc8174)
- [RFC 3339: Date and Time on the Internet: Timestamps](https://www.rfc-editor.org/rfc/rfc3339)
- [RFC 3986: Uniform Resource Identifier: Generic Syntax](https://www.rfc-editor.org/rfc/rfc3986)
- [RFC 9562: Universally Unique IDentifiers](https://www.rfc-editor.org/rfc/rfc9562)
- [TOML v1.0.0](https://toml.io/en/v1.0.0)
- [JSON](https://www.json.org/json-en.html)
- [YAML 1.2.2](https://yaml.org/spec/1.2.2/)
- [NO_COLOR](https://no-color.org/)

### 13.2 Rust Toolchain and Test Runner

- [Rust releases](https://blog.rust-lang.org/releases/)
- [Rustup overrides and `rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html)
- [Cargo manifest reference: `rust-version`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field)
- [cargo-nextest: Running tests](https://nexte.st/docs/running/)
- [cargo-nextest: Repository configuration](https://nexte.st/docs/configuration/)
- [cargo-nextest: Configuration reference](https://nexte.st/docs/configuration/reference/)

### 13.3 Development Environment

- [Nix flakes](https://nix.dev/concepts/flakes)
- [Nix flake command reference](https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-flake.html)
- [Nix develop command reference](https://nix.dev/manual/nix/stable/command-ref/new-cli/nix3-develop.html)
- [direnv](https://direnv.net/)
- [nix-direnv `use flake`](https://github.com/nix-community/nix-direnv)

### 13.4 Cloudflare R2 and API Tokens

- [Cloudflare R2 public buckets](https://developers.cloudflare.com/r2/buckets/public-buckets/)
- [Cloudflare R2 Wrangler commands](https://developers.cloudflare.com/r2/reference/wrangler-commands/)
- [Cloudflare R2 upload objects](https://developers.cloudflare.com/r2/objects/upload-objects/)
- [Cloudflare API: Zones](https://developers.cloudflare.com/api/resources/zones/)
- [Cloudflare API token permissions](https://developers.cloudflare.com/fundamentals/api/reference/permissions/)
- [Cloudflare create API token](https://developers.cloudflare.com/fundamentals/api/get-started/create-token/)
- [Cloudflare DNS records](https://developers.cloudflare.com/dns/manage-dns-records/how-to/create-dns-records/)
- [Cloudflare 1.1.1.1 DNS over TLS](https://developers.cloudflare.com/1.1.1.1/encryption/dns-over-tls/)
- [AliDNS Public DNS](https://www.alidns.com/)

### 13.5 Gitee Release Delivery

- [Gitee OpenAPI v5 Swagger](https://gitee.com/api/v5/swagger)
- [Gitee Release: 什么是 Release（发行版）？](https://help.gitee.com/repository/release/what-is-release)
- [Gitee Release: 创建 Release（发行版）](https://help.gitee.com/repository/release/create)
- [Gitee platform](https://gitee.com/)

### 13.6 sing-box, Clients, and Rule Sets

- [sing-box project](https://sing-box.sagernet.org/)
- [sing-box releases](https://github.com/SagerNet/sing-box/releases)
- [sing-box v1.13.13 release](https://github.com/SagerNet/sing-box/releases/tag/v1.13.13)
- [sing-box graphical clients](https://sing-box.sagernet.org/clients/)
- [sing-box graphical clients: General Remote Profile](https://sing-box.sagernet.org/clients/general/)
- [sing-box for Android](https://sing-box.sagernet.org/clients/android/)
- [sing-box for Apple platforms](https://sing-box.sagernet.org/clients/apple/)
- [sing-box configuration](https://sing-box.sagernet.org/configuration/)
- [sing-box rule set](https://sing-box.sagernet.org/configuration/rule-set/)
- [sing-box TUN inbound](https://sing-box.sagernet.org/configuration/inbound/tun/)
- [sing-box mixed inbound](https://sing-box.sagernet.org/configuration/inbound/mixed/)
- [sing-box DNS](https://sing-box.sagernet.org/configuration/dns/)
- [sing-box route](https://sing-box.sagernet.org/configuration/route/)
- [sing-box route rule](https://sing-box.sagernet.org/configuration/route/rule/)
- [sing-box VLESS inbound](https://sing-box.sagernet.org/configuration/inbound/vless/)
- [sing-box VLESS outbound](https://sing-box.sagernet.org/configuration/outbound/vless/)
- [sing-box URLTest outbound](https://sing-box.sagernet.org/configuration/outbound/urltest/)
- [sing-box shared TLS fields](https://sing-box.sagernet.org/configuration/shared/tls/)
- [Dreista sing-box-rule-set-cn](https://github.com/Dreista/sing-box-rule-set-cn)
- [felixonmars dnsmasq-china-list](https://github.com/felixonmars/dnsmasq-china-list)
- [misakaio chnroutes2](https://github.com/misakaio/chnroutes2)

### 13.7 Clash Verge Rev and mihomo

- [Clash Verge Rev releases](https://github.com/clash-verge-rev/clash-verge-rev/releases)
- [Clash Verge Rev documentation](https://www.clashverge.dev/)
- [Clash Verge Rev URL Schemes](https://www.clashverge.dev/guide/url_schemes.html)
- [Clash Verge Rev rules](https://www.clashverge.dev/guide/rules.html)
- [Clash Verge Rev configuration extension](https://www.clashverge.dev/guide/extend.html)
- [mihomo releases](https://github.com/MetaCubeX/mihomo/releases)
- [mihomo documentation](https://wiki.metacubex.one/en/)
- [mihomo general configuration](https://wiki.metacubex.one/en/config/general/)
- [mihomo DNS configuration](https://wiki.metacubex.one/en/config/dns/)
- [mihomo TUN inbound](https://wiki.metacubex.one/en/config/inbound/tun/)
- [mihomo VLESS proxy](https://wiki.metacubex.one/en/config/proxies/vless/)
- [mihomo proxy groups](https://wiki.metacubex.one/en/config/proxy-groups/)
- [mihomo URLTest proxy group](https://wiki.metacubex.one/en/config/proxy-groups/url-test/)
- [mihomo route rules](https://wiki.metacubex.one/en/config/rules/)
- [MetaCubeX meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat)

### 13.8 System Services, Runtime Probing, and Shell Primitives

- [systemd.service manual](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html)
- [launchd.plist manual](https://www.manpagez.com/man/5/launchd.plist/)
- [OpenSSH manual pages](https://www.openssh.com/manual.html)
- [curl command line tool](https://curl.se/docs/manpage.html)
- [curl SOCKS proxy documentation](https://everything.curl.dev/usingcurl/proxies/socks.html)
