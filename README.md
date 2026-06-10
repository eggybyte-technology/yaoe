<p align="center">
  <img src="assets/color-logo.svg" alt="YAOE logo" width="180" />
</p>

<h1 align="center">YAOE</h1>

<p align="center">
  Single-operator egress and profile-delivery controller for sing-box Reality deployments.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License: Apache-2.0" /></a>
  <img src="https://img.shields.io/badge/rust-1.96.0-orange.svg" alt="Rust 1.96.0" />
  <img src="https://img.shields.io/badge/sing--box-1.13.13-green.svg" alt="sing-box 1.13.13" />
  <img src="https://img.shields.io/badge/status-v0.0.1-informational.svg" alt="Status v0.0.1" />
</p>

YAOE provisions public Linux amd64 or arm64 egress servers, publishes Linux/macOS
service bootstrap scripts, publishes Clash Verge Rev and sing-box client
profiles, and prints exact client entrypoints from one local
`.yaoe/yaoe.toml` file.

The authoritative product contract is [`docs/design.md`](docs/design.md).

## Delivery Model

YAOE uses three public delivery surfaces:

```text
Gitee Release attachments
  -> sing-box runtime archives
  -> mirrored CN direct SRS rule sets

Gitee repository raw files
  -> Linux/macOS service install scripts
  -> Linux/macOS service update scripts

Cloudflare R2 public bucket
  -> clash-verge.yaml
  -> Linux/macOS service JSON configs
  -> iOS/Android sing-box Remote Profile JSON configs
```

Client config URLs are guarded by the local path credential
`credential.config_key`:

```text
https://<delivery-domain>/config/<config-key>/<profile-file>
```

## Supported Entrypoints

| Role                  | Entrypoint                                        |
| --------------------- | ------------------------------------------------- |
| Controller            | Linux CLI named `yaoe`                            |
| Managed egress server | Linux amd64 or arm64 with systemd                 |
| Windows desktop GUI   | Clash Verge Rev importing `clash-verge.yaml`      |
| macOS desktop GUI     | Clash Verge Rev importing `clash-verge.yaml`      |
| Linux desktop GUI     | Clash Verge Rev importing `clash-verge.yaml`      |
| macOS desktop service | YAOE-installed sing-box launchd service           |
| Linux desktop service | YAOE-installed sing-box systemd service           |
| iOS / iPadOS          | Official sing-box graphical client Remote Profile |
| Android               | Official sing-box graphical client Remote Profile |

Linux and macOS service scripts detect CPU architecture. Users pass no
architecture parameters.

## Requirements

The controller machine requires Linux, Nix flakes, direnv, OpenSSH, Git,
Wrangler, Rust `1.96.0`, cargo-nextest, sing-box `1.13.13`, and mihomo
`1.19.27`. The default Nix devShell provides these tools.

External infrastructure requirements:

- root SSH access to each managed Linux amd64 or arm64 server
- a Gitee token that can create/update the delivery repository, branch `main`,
  release, and release attachments
- a Cloudflare API token that can manage the R2 bucket, object upload,
  custom domain, DNS, and public fetch operations
- one R2 custom domain for published config objects

## First-Time Flow

Load the dev environment:

```bash
direnv allow
```

Install the CLI:

```bash
cargo install --path crates/yaoe-cli --locked --force
```

Create the local layout and sample config:

```bash
yaoe init
```

Edit `.yaoe/yaoe.toml` by replacing all placeholders.

Ensure the Cloudflare API token can make the R2 bucket, object upload,
custom-domain, DNS, and public fetch operations in `docs/design.md` succeed.

Ensure the Gitee token can create or update the delivery repository, branch
`main`, release, and release attachments.

Run the controller flow:

```bash
yaoe check
yaoe publish delivery
yaoe apply
yaoe status
yaoe health
yaoe client
```

`yaoe client` is the only supported client-entrypoint derivation command.

## Client Use

Windows users install Clash Verge Rev and import the
`clash-verge remote-profile` URL or `clash-verge import-url` printed by
`yaoe client`.

macOS users have two supported entrypoints printed by `yaoe client`:
`clash-verge` GUI profile and `macos sing-box` service commands.

Linux users have two supported entrypoints printed by `yaoe client`:
`clash-verge` GUI profile and `linux sing-box` service commands.

iOS / iPadOS users import the `ios remote-profile` URL from `yaoe client` into
the official sing-box graphical client as a Remote Profile.

Android users import the `android remote-profile` URL from `yaoe client` into
the official sing-box graphical client as a Remote Profile.

Generated configs implement IPv4 egress semantics: private/local traffic,
configured direct CIDRs, managed-server endpoint IPs, and CN allowlist traffic
are direct; remaining public IPv4 traffic uses proxy aggregation.

Clash Verge Rev users edit no rule files, merge files, script files, or
subscription conversion settings.

## Publication Commands

Run separated publication commands for diagnostics in this order:

```bash
yaoe publish bootstrap
yaoe publish runtime
yaoe publish config
```

The aggregate command runs all three:

```bash
yaoe publish delivery
```

## Server Operations

```bash
yaoe apply
yaoe apply <server>
yaoe status
yaoe status <server>
yaoe health
yaoe health <server>
```

A successful health check validates remote systemd state, remote config
validity, remote TCP listen state, and a local SOCKS5 Reality probe through
the managed server.

## Credential Rotation

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

Rotation commands update `.yaoe/yaoe.toml` locally and do not contact external
providers.

## Development Gates

Run the repository functional gate from the repository root after direnv has
loaded the devShell:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo install --path crates/yaoe-cli --locked --force
yaoe --help
```

When changing `flake.nix`, `flake.lock`, `rust-toolchain.toml`, or `.envrc`,
validate the development shell:

```bash
nix develop -c true
```

## Acceptance Validation

Real acceptance validation requires `.yaoe/yaoe.toml` to contain real provider
credentials, real Gitee coordinates, real Cloudflare R2 coordinates, and at
least one reachable managed Linux amd64 or arm64 server over root SSH.

Run:

```bash
cargo install --path crates/yaoe-cli --locked --force
cargo nextest run -p yaoe-controller -P acceptance --run-ignored=only acceptance_delivery
```

The acceptance flow executes:

```text
yaoe check
yaoe publish delivery
yaoe apply
yaoe status
yaoe health
yaoe client
```

## Version Pins

| Component                 | Version   |
| ------------------------- | --------- |
| YAOE product revision     | `v0.0.1`  |
| Rust                      | `1.96.0`  |
| sing-box                  | `1.13.13` |
| mihomo validation         | `1.19.27` |
| Clash Verge Rev reference | `v2.5.1`  |

## License

YAOE source code is licensed under the [Apache License 2.0](LICENSE).

YAOE may download, mirror, or reference third-party runtime artifacts such as
sing-box and rule-set files. Those artifacts remain under their respective
upstream licenses.

## Star History

[![Star History Chart](https://api.star-history.com/chart?repos=eggybyte-technology/yaoe&type=date&logscale&legend=top-left)](https://www.star-history.com/?type=date&repos=eggybyte-technology%2Fyaoe)
