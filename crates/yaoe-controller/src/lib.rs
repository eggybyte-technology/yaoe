//! Command orchestration for YAOE v0.0.1.

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::thread;
use std::time::Duration;

mod deps;
mod logging;
mod system;

pub use deps::RuntimeDeps;
pub use system::{
    LocalMihomo, LocalSingBox, ProbeFailure, ProbeRunResult, ProbeSuccess, PublicConfigFetcher,
    RealityKeypairGenerator, ReqwestPublicConfigFetcher, SystemLocalMihomo, SystemLocalSingBox,
    SystemRealityKeypairGenerator,
};

use crate::logging::{info, ok, one_line, progress, record_event as log_event, tail_lines, warn};
use yaoe_cloudflare::{R2Wrangler, public_config_object_key, public_config_url};
use yaoe_config::{
    ClientEntrypointParts, Config, atomic_update_reality_keypair, atomic_update_toml_field,
    derive_reality_public_key, generate_config_key, generate_reality_short_id,
    generate_server_port, generate_vless_uuid, load_and_validate, parse_for_client_entrypoints,
};
use yaoe_gitee::{
    BootstrapFile, GiteeDelivery, ReleaseAsset, ensure_bootstrap_branch, ensure_release,
    ensure_repository, publish_bootstrap_files, publish_release_assets,
};
use yaoe_home::{
    CLOUDFLARE_PUBLIC_FETCH_ATTEMPTS, CLOUDFLARE_PUBLIC_FETCH_INTERVAL_SECONDS, CONFIG_VARIANTS,
    GITEE_BOOTSTRAP_BRANCH, HEALTH_PROBE_BIND_HOST, HEALTH_PROBE_CURL_PROXY_KIND,
    HEALTH_PROBE_PORT_RETRY_LIMIT, HEALTH_PROBE_URL, HomePaths, R2_CUSTOM_DOMAIN_MIN_TLS,
    R2_JSON_CONTENT_TYPE, R2_YAML_CONTENT_TYPE, REMOTE_JOURNAL_TAIL_LINES, SERVICE_SCRIPT_TARGETS,
    SING_BOX_VERSION, YaoeError, YaoeResult, atomic_write, config_variant, ensure_home, init_home,
    managed_server_runtime_variant, resolve_home, script_extension, sha256_hex,
    sing_box_version_line, validate_home_layout,
};
use yaoe_package::{PackageBuildInput, build_server_package};
use yaoe_render::{
    ClientPlatform, ClientRenderInput, HealthProbeRenderInput, ServerRenderInput,
    render_clash_verge_profile, render_client_config, render_health_probe_config,
    render_install_script, render_server_config, render_update_script,
};
use yaoe_rules::ensure_srs_cache;
use yaoe_upstream::{ensure_runtime_artifacts, resolve_server_runtime, runtime_assets};

pub fn cmd_init(home: Option<&Path>, deps: &RuntimeDeps) -> YaoeResult<()> {
    let paths = resolve_home(home);
    init_home(&paths)?;
    if paths.config.exists() {
        println!("initialized {}", paths.root.display());
        return Ok(());
    }
    let (private_key, public_key) = deps.reality_keypair.generate()?;
    let derived = derive_reality_public_key(&private_key)?;
    if derived != public_key {
        return Err(YaoeError::Config(
            "generated Reality public key mismatch".into(),
        ));
    }
    let vless_uuid = generate_vless_uuid();
    let config_key = generate_config_key();
    let reality_short_id = generate_reality_short_id();
    let server_port = generate_server_port();
    let sample = format!(
        r#"[ssh]
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
vless_uuid = "{vless_uuid}"
config_key = "{config_key}"
reality_private_key = "{private_key}"
reality_short_id = "{reality_short_id}"

[reality]
handshake_server = "www.cloudflare.com"

[route]
direct_cidrs = ["100.64.0.0/10"]

[server.hk]
ssh = "root@203.0.113.20"
ip = "203.0.113.20"
port = {}
"#,
        server_port
    );
    atomic_write(&paths.config, sample.as_bytes(), 0o600)?;
    println!("created {}", paths.config.display());
    println!("config_key {config_key}");
    println!("vless_uuid {vless_uuid}");
    println!("reality_private_key {private_key}");
    println!("reality_public_key {derived}");
    println!("reality_short_id {reality_short_id}");
    Ok(())
}

pub fn cmd_check(home: Option<&Path>) -> YaoeResult<()> {
    let paths = resolve_home(home);
    if !paths.root.is_dir() {
        return Err(YaoeError::State(format!(
            "{} is not an initialized YAOE home",
            paths.root.display()
        )));
    }
    let config = load_and_validate(&paths.config)?;
    validate_home_layout(&paths)?;
    let _ = derive_reality_public_key(&config.credential.reality_private_key)?;
    log_event("check", "config", &[("ok", "true".to_string())]);
    println!("check ok: {} server(s)", config.server.len());
    Ok(())
}

pub fn cmd_client(home: Option<&Path>) -> YaoeResult<()> {
    let paths = resolve_home(home);
    let text = fs::read_to_string(&paths.config)
        .map_err(|e| YaoeError::State(format!("failed to read {}: {e}", paths.config.display())))?;
    let parts = parse_for_client_entrypoints(&text)?;
    print!("{}", render_client_entrypoints(&parts));
    Ok(())
}

pub fn cmd_rotate_config_key(home: Option<&Path>) -> YaoeResult<()> {
    let paths = resolve_home(home);
    let config_key = generate_config_key();
    atomic_update_toml_field(&paths.config, "credential", "config_key", &config_key)?;
    println!("config_key {config_key}");
    println!("next yaoe publish config");
    Ok(())
}

pub fn cmd_rotate_vless_uuid(home: Option<&Path>) -> YaoeResult<()> {
    let paths = resolve_home(home);
    let vless_uuid = generate_vless_uuid();
    atomic_update_toml_field(&paths.config, "credential", "vless_uuid", &vless_uuid)?;
    println!("vless_uuid {vless_uuid}");
    println!("next yaoe apply");
    println!("next yaoe publish config");
    Ok(())
}

pub fn cmd_rotate_reality_keypair(home: Option<&Path>, deps: &RuntimeDeps) -> YaoeResult<()> {
    let paths = resolve_home(home);
    let (private_key, public_key) = deps.reality_keypair.generate()?;
    if derive_reality_public_key(&private_key)? != public_key {
        return Err(YaoeError::Config(
            "generated Reality public key mismatch".into(),
        ));
    }
    let reality_short_id = generate_reality_short_id();
    atomic_update_reality_keypair(&paths.config, &private_key, &reality_short_id)?;
    println!("reality_private_key {private_key}");
    println!("reality_public_key {public_key}");
    println!("reality_short_id {reality_short_id}");
    println!("next yaoe apply");
    println!("next yaoe publish config");
    Ok(())
}

pub fn cmd_apply(home: Option<&Path>, server: Option<&str>, deps: &RuntimeDeps) -> YaoeResult<()> {
    progress("apply: loading configuration");
    let paths = resolve_home(home);
    let config = load_and_validate(&paths.config)?;
    let selected = selected_server_names(&config, server)?;
    ensure_home(&paths)?;
    info(
        "apply",
        "selected servers",
        &[("count", selected.len().to_string())],
    );
    for name in selected {
        let server_config = &config.server[&name];
        let key = server_ssh_key(&config, server_config.key.as_deref())?;
        progress(format!(
            "apply:{name}: detecting managed-server architecture"
        ));
        let runtime_variant = detect_server_runtime_variant(deps, &server_config.ssh, &key, &name)?;
        progress(format!("apply:{name}: rendering server config"));
        let config_json = render_server_config(&ServerRenderInput {
            server_name: name.clone(),
            endpoint_ip: server_config.ip.clone(),
            port: server_config.port,
            vless_uuid: config.credential.vless_uuid.clone(),
            reality_private_key: config.credential.reality_private_key.clone(),
            reality_short_id: config.credential.reality_short_id.clone(),
            handshake_server: config.reality.handshake_server.clone(),
            handshake_port: config.reality.handshake_port,
        })?;
        progress(format!(
            "apply:{name}: resolving managed-server sing-box runtime"
        ));
        let runtime = resolve_server_runtime(
            &paths,
            runtime_variant,
            &config.gitee.owner,
            &config.gitee.repo,
            deps.upstream_fetcher.as_ref(),
        )?;
        let sing_box_bytes = fs::read(&runtime.path)
            .map_err(|e| YaoeError::State(format!("read {}: {e}", runtime.path.display())))?;
        progress(format!("apply:{name}: building transfer package"));
        let pkg = build_server_package(
            &paths,
            &PackageBuildInput {
                server_name: name.clone(),
                runtime_variant: runtime_variant.to_string(),
                config_sha256: sha256_hex(config_json.as_bytes()),
                config_json,
                sing_box_sha256: runtime.sha256.clone(),
                sing_box_bytes: sing_box_bytes.clone(),
            },
        )?;
        let nonce = format!(
            "{}-{}",
            std::process::id(),
            yaoe_home::digest_prefix(&pkg.package_sha256)
        );
        let remote_pkg = format!("/tmp/yaoe-{name}-{nonce}.tar.gz");
        let remote_dir = format!("/tmp/yaoe-install-{name}-{nonce}");
        progress(format!("apply:{name}: uploading package over SSH"));
        retry_package_upload(
            deps,
            &server_config.ssh,
            pkg.path
                .to_str()
                .ok_or_else(|| YaoeError::State("package path is not UTF-8".into()))?,
            &remote_pkg,
            &key,
            &name,
        )?;
        progress(format!("apply:{name}: running remote installer"));
        let command = format!(
            "set +e; rm -rf '{remote_dir}' || exit 97; mkdir -p '{remote_dir}' || exit 97; tar -xzf '{remote_pkg}' -C '{remote_dir}' || exit 97; cd '{remote_dir}/yaoe-server-package' || exit 97; sh ./install.sh; status=$?; if [ \"$status\" -eq 0 ]; then rm -f '{remote_pkg}'; rm -rf '{remote_dir}'; fi; exit \"$status\""
        );
        let output = deps
            .ssh
            .run_as_root_raw(&server_config.ssh, &command, &key)?;
        if output.status == 97 {
            return Err(YaoeError::Ssh(format!(
                "remote package extraction failed for {name}: {}",
                output.stderr.trim()
            )));
        }
        if output.status != 0 {
            return Err(YaoeError::Installer(format!(
                "target installer failed for {name}: {}",
                output.stderr.trim()
            )));
        }
        progress(format!("apply:{name}: verifying service active state"));
        require_remote_active(deps, &server_config.ssh, &key, &name)?;
        log_event("apply", &name, &[("service", "active".to_string())]);
    }
    Ok(())
}

fn detect_server_runtime_variant(
    deps: &RuntimeDeps,
    destination: &str,
    key: &str,
    name: &str,
) -> YaoeResult<&'static str> {
    let os = run_remote_status_command(deps, destination, "uname -s", key, "apply", name)?;
    if os.status != 0 {
        return Err(YaoeError::Installer(format!(
            "apply/{name}: remote OS detection failed status={} stderr={}",
            os.status,
            os.stderr.trim()
        )));
    }
    let cpu = run_remote_status_command(deps, destination, "uname -m", key, "apply", name)?;
    if cpu.status != 0 {
        return Err(YaoeError::Installer(format!(
            "apply/{name}: remote CPU detection failed status={} stderr={}",
            cpu.status,
            cpu.stderr.trim()
        )));
    }
    let variant = managed_server_runtime_variant(&os.stdout, &cpu.stdout).ok_or_else(|| {
        YaoeError::Installer(format!(
            "apply/{name}: unsupported managed server platform os={} arch={}",
            one_line(os.stdout.trim()),
            one_line(cpu.stdout.trim())
        ))
    })?;
    ok(
        &format!("apply:{name}"),
        "managed-server architecture detected",
        &[
            ("os", os.stdout.trim().to_string()),
            ("arch", cpu.stdout.trim().to_string()),
            ("runtime", variant.to_string()),
        ],
    );
    Ok(variant)
}

pub fn cmd_publish_bootstrap(home: Option<&Path>, deps: &RuntimeDeps) -> YaoeResult<()> {
    progress("publish bootstrap: loading configuration");
    let paths = resolve_home(home);
    let config = load_and_validate(&paths.config)?;
    ensure_home(&paths)?;
    let delivery = gitee_delivery(&config);
    progress("publish bootstrap: ensuring Gitee repository");
    ensure_repository(&delivery, deps.gitee.as_ref())?;
    progress("publish bootstrap: rendering desktop scripts");
    let files = render_bootstrap_files(&paths, &config)?;
    progress("publish bootstrap: ensuring Gitee main branch baseline");
    ensure_bootstrap_branch(
        &paths,
        &delivery,
        deps.gitee.as_ref(),
        deps.git.as_ref(),
        &files,
    )?;
    info(
        "publish.bootstrap",
        "publishing script files",
        &[("count", files.len().to_string())],
    );
    publish_bootstrap_files(
        &paths,
        &delivery,
        deps.gitee.as_ref(),
        deps.git.as_ref(),
        &files,
    )?;
    log_event(
        "publish",
        "bootstrap",
        &[
            (
                "repo",
                format!("{}/{}", config.gitee.owner, config.gitee.repo),
            ),
            ("branch", yaoe_home::GITEE_BOOTSTRAP_BRANCH.to_string()),
            ("files", files.len().to_string()),
        ],
    );
    Ok(())
}

pub fn cmd_publish_runtime(home: Option<&Path>, deps: &RuntimeDeps) -> YaoeResult<()> {
    progress("publish runtime: loading configuration");
    let paths = resolve_home(home);
    let config = load_and_validate(&paths.config)?;
    ensure_home(&paths)?;
    let delivery = gitee_delivery(&config);
    progress("publish runtime: ensuring Gitee repository");
    ensure_repository(&delivery, deps.gitee.as_ref())?;
    progress("publish runtime: rendering bootstrap baseline scripts");
    let files = render_bootstrap_files(&paths, &config)?;
    progress("publish runtime: ensuring Gitee main branch baseline");
    ensure_bootstrap_branch(
        &paths,
        &delivery,
        deps.gitee.as_ref(),
        deps.git.as_ref(),
        &files,
    )?;
    progress("publish runtime: ensuring fixed Gitee release");
    let release = ensure_release(&delivery, deps.gitee.as_ref())?;
    progress("publish runtime: resolving upstream runtime artifacts");
    let _ = ensure_runtime_artifacts(&paths, deps.upstream_fetcher.as_ref())?;
    progress("publish runtime: resolving CN direct SRS cache");
    let _ = ensure_srs_cache(
        &paths,
        deps.srs_fetcher.as_ref(),
        deps.srs_validator.as_ref(),
    )?;
    let assets: Vec<ReleaseAsset> = runtime_assets(&paths)
        .into_iter()
        .map(|artifact| ReleaseAsset {
            name: artifact.asset_name,
            path: artifact.cache_path,
        })
        .collect();
    info(
        "publish.runtime",
        "publishing release assets",
        &[("count", assets.len().to_string())],
    );
    let statuses =
        publish_release_assets(&paths, &delivery, deps.gitee.as_ref(), &release, &assets)?;
    for (asset, status) in statuses {
        let action = match status {
            yaoe_gitee::ReleaseAssetStatus::LocalMarkerSkip => "skip-local-marker",
            yaoe_gitee::ReleaseAssetStatus::RemoteExists => "skip-remote-exists",
            yaoe_gitee::ReleaseAssetStatus::Uploaded => "upload",
        };
        log_event(
            "publish",
            "runtime",
            &[("asset", asset), ("action", action.to_string())],
        );
    }
    Ok(())
}

pub fn cmd_publish_config(home: Option<&Path>, deps: &RuntimeDeps) -> YaoeResult<()> {
    progress("publish config: loading configuration");
    let paths = resolve_home(home);
    let config = load_and_validate(&paths.config)?;
    ensure_home(&paths)?;
    progress("publish config: checking sing-box from PATH");
    deps.local_sing_box.require_version()?;
    progress("publish config: checking mihomo from PATH");
    deps.local_mihomo.require_version()?;
    info(
        "publish.config",
        "resolving cloudflare zone",
        &[("domain", config.cloudflare.delivery_domain.clone())],
    );
    let zone_id = deps
        .cloudflare
        .resolve_zone_id(&config.cloudflare.delivery_domain)?;
    info(
        "publish.config",
        "checking r2 bucket",
        &[("bucket", config.cloudflare.r2_bucket.clone())],
    );
    if !deps.r2.bucket_exists(
        &config.cloudflare.account_id,
        &config.cloudflare.token,
        &config.cloudflare.r2_bucket,
    )? {
        info(
            "publish.config",
            "creating r2 bucket",
            &[("bucket", config.cloudflare.r2_bucket.clone())],
        );
        deps.r2.create_bucket(
            &config.cloudflare.account_id,
            &config.cloudflare.token,
            &config.cloudflare.r2_bucket,
        )?;
    }
    info(
        "publish.config",
        "checking r2 custom domain",
        &[("domain", config.cloudflare.delivery_domain.clone())],
    );
    match deps.r2.domain_state(
        &config.cloudflare.account_id,
        &config.cloudflare.token,
        &config.cloudflare.r2_bucket,
        &config.cloudflare.delivery_domain,
    )? {
        None => {
            info(
                "publish.config",
                "adding r2 custom domain",
                &[("domain", config.cloudflare.delivery_domain.clone())],
            );
            deps.r2.add_domain(
                &config.cloudflare.account_id,
                &config.cloudflare.token,
                &config.cloudflare.r2_bucket,
                &config.cloudflare.delivery_domain,
                &zone_id,
            )?;
        }
        Some(state) if state.min_tls.as_deref() != Some(R2_CUSTOM_DOMAIN_MIN_TLS) => {
            info(
                "publish.config",
                "updating r2 custom domain minimum tls",
                &[("min_tls", R2_CUSTOM_DOMAIN_MIN_TLS.to_string())],
            );
            deps.r2.update_domain_tls(
                &config.cloudflare.account_id,
                &config.cloudflare.token,
                &config.cloudflare.r2_bucket,
                &config.cloudflare.delivery_domain,
            )?;
        }
        Some(_) => {}
    }
    progress("publish config: rendering and checking seven config variants");
    render_platform_configs(
        &paths,
        &config,
        deps.local_sing_box.as_ref(),
        deps.local_mihomo.as_ref(),
    )?;
    upload_config_objects(&paths, &config, deps.r2.as_ref())?;
    progress("publish config: validating public R2 config URLs");
    validate_public_configs(
        &paths,
        &config,
        deps.local_sing_box.as_ref(),
        deps.local_mihomo.as_ref(),
        deps.public_config_fetcher.as_ref(),
    )?;
    for platform in CONFIG_VARIANTS {
        let url = public_config_url(
            &config.cloudflare.delivery_domain,
            &config.credential.config_key,
            platform,
        );
        println!("config {platform} {url}");
    }
    Ok(())
}

pub fn cmd_publish_delivery(home: Option<&Path>, deps: &RuntimeDeps) -> YaoeResult<()> {
    progress("publish delivery: starting bootstrap, runtime, config sequence");
    cmd_publish_bootstrap(home, deps)?;
    cmd_publish_runtime(home, deps)?;
    cmd_publish_config(home, deps)
}

pub fn cmd_status(home: Option<&Path>, server: Option<&str>, deps: &RuntimeDeps) -> YaoeResult<()> {
    let paths = resolve_home(home);
    let config = load_and_validate(&paths.config)?;
    for name in selected_server_names(&config, server)? {
        let summary = remote_status_checks("status", deps, &config, &name)?;
        println!(
            "status {} active={} enabled={} pid={} version={} config=ok listen={}:{}",
            summary.server,
            summary.active,
            summary.enabled,
            summary.main_pid,
            SING_BOX_VERSION,
            summary.listen_ip,
            summary.listen_port
        );
    }
    Ok(())
}

pub fn cmd_health(home: Option<&Path>, server: Option<&str>, deps: &RuntimeDeps) -> YaoeResult<()> {
    let paths = resolve_home(home);
    let config = load_and_validate(&paths.config)?;
    let selected = selected_server_names(&config, server)?;
    for name in &selected {
        info(
            &format!("health:{name}"),
            "checking local sing-box",
            &[("expected", SING_BOX_VERSION.to_string())],
        );
    }
    deps.local_sing_box.require_version()?;
    for name in &selected {
        ok(
            &format!("health:{name}"),
            "local sing-box version verified",
            &[
                ("expected", SING_BOX_VERSION.to_string()),
                ("actual", SING_BOX_VERSION.to_string()),
            ],
        );
    }
    for name in selected {
        info(
            &format!("health:{name}"),
            "running remote status prerequisites",
            &[("server", name.clone())],
        );
        let summary = remote_status_checks("health", deps, &config, &name)?;
        ok(
            &format!("health:{name}"),
            "remote status prerequisites passed",
            &[("server", name.clone())],
        );
        let success = run_health_probe_for_server(&paths, deps, &config, &summary)?;
        println!(
            "health {} status={} url={} via={}:{} elapsed_ms={}",
            summary.server,
            success.status,
            HEALTH_PROBE_URL,
            summary.listen_ip,
            summary.listen_port,
            success.elapsed_ms
        );
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct RemoteStatusSummary {
    server: String,
    destination: String,
    key: String,
    listen_ip: String,
    listen_port: u16,
    active: String,
    enabled: String,
    main_pid: u32,
}

fn remote_status_checks(
    command_name: &str,
    deps: &RuntimeDeps,
    config: &Config,
    name: &str,
) -> YaoeResult<RemoteStatusSummary> {
    let server_config = &config.server[name];
    let key = server_ssh_key(config, server_config.key.as_deref())?;
    info(
        &format!("{command_name}:{name}"),
        "checking ssh connectivity",
        &[("target", server_config.ssh.clone()), ("key", key.clone())],
    );

    let active_output = run_remote_status_command(
        deps,
        &server_config.ssh,
        &format!("systemctl is-active yaoe-{name}.service"),
        &key,
        command_name,
        name,
    )?;
    let active = active_output.stdout.trim().to_string();
    if active_output.status != 0 || active != "active" {
        return Err(YaoeError::Installer(format!(
            "{command_name}/{name}: systemctl is-active failed status={} stdout={} stderr={}",
            active_output.status,
            active,
            active_output.stderr.trim()
        )));
    }

    let enabled_output = run_remote_status_command(
        deps,
        &server_config.ssh,
        &format!("systemctl is-enabled yaoe-{name}.service"),
        &key,
        command_name,
        name,
    )?;
    let enabled = if enabled_output.stdout.trim().is_empty() {
        enabled_output.stderr.trim().to_string()
    } else {
        enabled_output.stdout.trim().to_string()
    };

    let main_pid = run_remote_status_command(
        deps,
        &server_config.ssh,
        &format!("systemctl show yaoe-{name}.service --property=MainPID --value"),
        &key,
        command_name,
        name,
    )?;
    let pid = main_pid.stdout.trim().parse::<u32>().unwrap_or(0);
    if main_pid.status != 0 || pid == 0 {
        return Err(YaoeError::Installer(format!(
            "{command_name}/{name}: MainPID validation failed status={} stdout={} stderr={}",
            main_pid.status,
            main_pid.stdout.trim(),
            main_pid.stderr.trim()
        )));
    }
    ok(
        &format!("{command_name}:{name}"),
        "systemd service active",
        &[
            ("service", format!("yaoe-{name}.service")),
            ("active", active.clone()),
            ("enabled", enabled.clone()),
            ("main_pid", pid.to_string()),
        ],
    );

    let version_output = run_remote_status_command(
        deps,
        &server_config.ssh,
        "/var/lib/yaoe/bin/sing-box version",
        &key,
        command_name,
        name,
    )?;
    let version = version_output.stdout.lines().next().unwrap_or_default();
    if version_output.status != 0 || version != sing_box_version_line() {
        return Err(YaoeError::Installer(format!(
            "{command_name}/{name}: remote sing-box version validation failed status={} stdout={} stderr={}",
            version_output.status,
            version_output.stdout.trim(),
            version_output.stderr.trim()
        )));
    }
    ok(
        &format!("{command_name}:{name}"),
        "sing-box version verified",
        &[
            ("expected", SING_BOX_VERSION.to_string()),
            ("actual", SING_BOX_VERSION.to_string()),
        ],
    );

    let check = run_remote_status_command(
        deps,
        &server_config.ssh,
        &format!("/var/lib/yaoe/bin/sing-box check -c /etc/yaoe/config/{name}.json"),
        &key,
        command_name,
        name,
    )?;
    if check.status != 0 {
        return Err(YaoeError::Installer(format!(
            "{command_name}/{name}: remote sing-box check failed status={} stderr={}",
            check.status,
            check.stderr.trim()
        )));
    }
    ok(
        &format!("{command_name}:{name}"),
        "config check passed",
        &[("path", format!("/etc/yaoe/config/{name}.json"))],
    );

    let listen = run_remote_status_command(
        deps,
        &server_config.ssh,
        &format!("ss -H -ltn sport = :{}", server_config.port),
        &key,
        command_name,
        name,
    )?;
    if listen.status != 0
        || !listen_output_matches(&listen.stdout, &server_config.ip, server_config.port)
    {
        return Err(YaoeError::Installer(format!(
            "{command_name}/{name}: remote listen validation failed status={} stdout={} stderr={}",
            listen.status,
            listen.stdout.trim(),
            listen.stderr.trim()
        )));
    }
    ok(
        &format!("{command_name}:{name}"),
        "tcp listen verified",
        &[(
            "listen",
            format!("{}:{}", server_config.ip, server_config.port),
        )],
    );

    log_event(
        command_name,
        name,
        &[
            ("active", active.clone()),
            ("enabled", enabled.clone()),
            ("version", SING_BOX_VERSION.to_string()),
            (
                "listen",
                format!("{}:{}", server_config.ip, server_config.port),
            ),
        ],
    );

    Ok(RemoteStatusSummary {
        server: name.to_string(),
        destination: server_config.ssh.clone(),
        key,
        listen_ip: server_config.ip.clone(),
        listen_port: server_config.port,
        active,
        enabled,
        main_pid: pid,
    })
}

fn run_remote_status_command(
    deps: &RuntimeDeps,
    destination: &str,
    command: &str,
    key: &str,
    command_name: &str,
    server: &str,
) -> YaoeResult<yaoe_ssh::RemoteCommandOutput> {
    let mut last_error = None;
    for attempt in 1..=3 {
        match deps.ssh.run_as_root_raw(destination, command, key) {
            Ok(output) if output.status == 255 => {
                let err_text = format!("ssh exited with status 255: {}", output.stderr.trim());
                warn(
                    &format!("{command_name}:{server}"),
                    "retrying ssh command",
                    &[
                        ("attempt", format!("{attempt}/3")),
                        ("command", command.to_string()),
                        ("error", one_line(&err_text)),
                    ],
                );
                last_error = Some(err_text);
                if attempt != 3 {
                    thread::sleep(Duration::from_secs(attempt * 2));
                }
            }
            Ok(output) => return Ok(output),
            Err(err @ YaoeError::Ssh(_)) => {
                let err_text = err.to_string();
                warn(
                    &format!("{command_name}:{server}"),
                    "retrying ssh command",
                    &[
                        ("attempt", format!("{attempt}/3")),
                        ("command", command.to_string()),
                        ("error", one_line(&err_text)),
                    ],
                );
                last_error = Some(err_text);
                if attempt != 3 {
                    thread::sleep(Duration::from_secs(attempt * 2));
                }
            }
            Err(err) => return Err(err),
        }
    }
    Err(YaoeError::Ssh(format!(
        "remote status command failed after retries for {server}: {}",
        last_error.as_deref().unwrap_or("<none>")
    )))
}

fn listen_output_matches(output: &str, ip: &str, port: u16) -> bool {
    if ip.parse::<std::net::Ipv4Addr>().is_err() {
        return false;
    };
    output.lines().any(|line| {
        line.contains(&format!("0.0.0.0:{port}")) || line.contains(&format!("{ip}:{port}"))
    })
}

fn run_health_probe_for_server(
    paths: &HomePaths,
    deps: &RuntimeDeps,
    config: &Config,
    summary: &RemoteStatusSummary,
) -> YaoeResult<ProbeSuccess> {
    let public_key = derive_reality_public_key(&config.credential.reality_private_key)?;
    let mut last_failure = None;
    for attempt in 1..=HEALTH_PROBE_PORT_RETRY_LIMIT {
        let probe_port = match reserve_probe_port() {
            Ok(port) => port,
            Err(detail) => {
                last_failure = Some(ProbeFailure {
                    stage: "bind",
                    curl_status: None,
                    curl_exit: None,
                    stderr_tail: String::new(),
                    detail,
                });
                continue;
            }
        };
        let rendered = render_health_probe_config(&HealthProbeRenderInput {
            endpoint_ip: summary.listen_ip.clone(),
            port: summary.listen_port,
            probe_port,
            vless_uuid: config.credential.vless_uuid.clone(),
            reality_public_key: public_key.clone(),
            reality_short_id: config.credential.reality_short_id.clone(),
            handshake_server: config.reality.handshake_server.clone(),
        })?;
        let path = paths.health_probe_path(&summary.server);
        atomic_write(&path, rendered.as_bytes(), 0o600)?;
        info(
            &format!("health.probe:{}", summary.server),
            "rendered probe config",
            &[("path", path.display().to_string())],
        );
        info(
            &format!("health.probe:{}", summary.server),
            "selected local probe port",
            &[
                ("bind", format!("{HEALTH_PROBE_BIND_HOST}:{probe_port}")),
                ("attempt", attempt.to_string()),
            ],
        );
        info(
            &format!("health.probe:{}", summary.server),
            "starting local sing-box",
            &[
                ("command", "run".to_string()),
                ("config", path.display().to_string()),
            ],
        );
        match deps
            .local_sing_box
            .run_health_probe(&path, probe_port, &summary.server)
        {
            Ok(success) => {
                ok(
                    &format!("health.probe:{}", summary.server),
                    "http probe passed",
                    &[
                        ("status", success.status.to_string()),
                        ("elapsed_ms", success.elapsed_ms.to_string()),
                        (
                            "via",
                            format!("{}:{}", summary.listen_ip, summary.listen_port),
                        ),
                        ("resolve", "remote_hostname_ipv4".to_string()),
                    ],
                );
                ok(
                    &format!("health.probe:{}", summary.server),
                    "stopped local sing-box",
                    &[("pid", success.pid.to_string())],
                );
                return Ok(success);
            }
            Err(failure)
                if matches!(failure.stage, "bind" | "startup")
                    && attempt < HEALTH_PROBE_PORT_RETRY_LIMIT =>
            {
                last_failure = Some(failure);
            }
            Err(failure) => {
                return Err(health_probe_error(deps, config, summary, failure));
            }
        }
    }
    Err(health_probe_error(
        deps,
        config,
        summary,
        last_failure.unwrap_or_else(|| ProbeFailure {
            stage: "bind",
            curl_status: None,
            curl_exit: None,
            stderr_tail: String::new(),
            detail: "probe port retry limit exhausted".into(),
        }),
    ))
}

fn reserve_probe_port() -> Result<u16, String> {
    let listener = TcpListener::bind((HEALTH_PROBE_BIND_HOST, 0))
        .map_err(|e| format!("bind {HEALTH_PROBE_BIND_HOST}:0: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("read local probe port: {e}"))?
        .port();
    drop(listener);
    Ok(port)
}

fn health_probe_error(
    deps: &RuntimeDeps,
    config: &Config,
    summary: &RemoteStatusSummary,
    failure: ProbeFailure,
) -> YaoeError {
    let journal_tail = remote_journal_tail(deps, summary);
    log_event(
        "health",
        &summary.server,
        &[
            ("probe", "failed".to_string()),
            ("stage", failure.stage.to_string()),
            ("proxy", HEALTH_PROBE_CURL_PROXY_KIND.to_string()),
            (
                "curl_status",
                failure.curl_status.clone().unwrap_or_else(|| "none".into()),
            ),
            (
                "curl_exit",
                failure
                    .curl_exit
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "none".into()),
            ),
        ],
    );
    YaoeError::HealthProbe(format!(
        "server={} endpoint={}:{} handshake={}:{} stage={} proxy={} resolve=remote_hostname_ipv4 curl_status={} curl_exit={} local_probe_stderr_tail={} remote_journal_tail={} detail={}",
        summary.server,
        summary.listen_ip,
        summary.listen_port,
        config.reality.handshake_server,
        config.reality.handshake_port,
        failure.stage,
        HEALTH_PROBE_CURL_PROXY_KIND,
        failure.curl_status.unwrap_or_else(|| "none".into()),
        failure
            .curl_exit
            .map(|code| code.to_string())
            .unwrap_or_else(|| "none".into()),
        one_line(&failure.stderr_tail),
        one_line(&journal_tail),
        one_line(&failure.detail)
    ))
}

fn remote_journal_tail(deps: &RuntimeDeps, summary: &RemoteStatusSummary) -> String {
    let command = format!(
        "journalctl -u yaoe-{}.service -n {} --no-pager",
        summary.server, REMOTE_JOURNAL_TAIL_LINES
    );
    match deps
        .ssh
        .run_as_root_raw(&summary.destination, &command, &summary.key)
    {
        Ok(output) if output.status == 0 => tail_lines(&output.stdout, REMOTE_JOURNAL_TAIL_LINES),
        Ok(output) => tail_lines(
            &format!("status={} stderr={}", output.status, output.stderr.trim()),
            REMOTE_JOURNAL_TAIL_LINES,
        ),
        Err(err) => format!("unreachable: {err}"),
    }
}

fn render_bootstrap_files(paths: &HomePaths, config: &Config) -> YaoeResult<Vec<BootstrapFile>> {
    let mut files = Vec::new();
    for target in SERVICE_SCRIPT_TARGETS {
        let install = render_install_script(config, target)?;
        let ext = script_extension(target).ok_or_else(|| {
            YaoeError::Internal(format!("unsupported service script target: {target}"))
        })?;
        let install_path = format!("install/{target}.{ext}");
        atomic_write(
            &paths.bootstrap_script_path("install", target),
            install.as_bytes(),
            0o644,
        )?;
        files.push(BootstrapFile {
            path: install_path,
            bytes: install.into_bytes(),
        });
    }
    for target in SERVICE_SCRIPT_TARGETS {
        let update = render_update_script(config, target)?;
        let ext = script_extension(target).ok_or_else(|| {
            YaoeError::Internal(format!("unsupported service script target: {target}"))
        })?;
        let update_path = format!("update/{target}.{ext}");
        atomic_write(
            &paths.bootstrap_script_path("update", target),
            update.as_bytes(),
            0o644,
        )?;
        files.push(BootstrapFile {
            path: update_path,
            bytes: update.into_bytes(),
        });
    }
    Ok(files)
}

fn retry_package_upload(
    deps: &RuntimeDeps,
    destination: &str,
    local_path: &str,
    remote_path: &str,
    key: &str,
    server_name: &str,
) -> YaoeResult<()> {
    let mut last_error = None;
    for attempt in 1..=5 {
        match deps.ssh.upload(destination, local_path, remote_path, key) {
            Ok(()) => return Ok(()),
            Err(err) => {
                let err_text = err.to_string();
                warn(
                    &format!("apply:{server_name}"),
                    "package upload failed",
                    &[
                        ("attempt", format!("{attempt}/5")),
                        ("error", one_line(&err_text)),
                    ],
                );
                last_error = Some(err_text);
                if attempt != 5 {
                    thread::sleep(Duration::from_secs(attempt * 2));
                }
            }
        }
    }
    Err(YaoeError::Ssh(format!(
        "package upload failed after retries for {server_name}: {}",
        last_error.as_deref().unwrap_or("<none>")
    )))
}

fn render_platform_configs(
    paths: &HomePaths,
    config: &Config,
    sing_box: &dyn LocalSingBox,
    mihomo: &dyn LocalMihomo,
) -> YaoeResult<()> {
    let input = ClientRenderInput {
        config: config.clone(),
    };
    for platform in CONFIG_VARIANTS {
        let rendered = if platform == "clash-verge" {
            progress(format!("publish config:{platform}: render YAML"));
            render_clash_verge_profile(&input)?
        } else {
            progress(format!("publish config:{platform}: render JSON"));
            let client_platform =
                ClientPlatform::from_config_platform(platform).ok_or_else(|| {
                    YaoeError::Internal(format!("unknown config platform {platform}"))
                })?;
            render_client_config(&input, client_platform)?
        };
        let path = paths.rendered_config_path(platform);
        atomic_write(&path, rendered.as_bytes(), 0o600)?;
        validate_rendered_config(platform, &path, sing_box, mihomo)?;
    }
    Ok(())
}

fn validate_rendered_config(
    platform: &str,
    path: &Path,
    sing_box: &dyn LocalSingBox,
    mihomo: &dyn LocalMihomo,
) -> YaoeResult<()> {
    if platform == "clash-verge" {
        progress(format!("publish config:{platform}: mihomo check YAML"));
        mihomo.check_config(path)
    } else {
        progress(format!("publish config:{platform}: sing-box check JSON"));
        sing_box.check_config(path)
    }
}

fn config_content_type(platform: &str) -> YaoeResult<&'static str> {
    match platform {
        "clash-verge" => Ok(R2_YAML_CONTENT_TYPE),
        _ if config_variant(platform).is_some() => Ok(R2_JSON_CONTENT_TYPE),
        _ => Err(YaoeError::Internal(format!(
            "unknown config platform {platform}"
        ))),
    }
}

fn upload_config_objects(
    paths: &HomePaths,
    config: &Config,
    r2: &dyn R2Wrangler,
) -> YaoeResult<()> {
    for platform in CONFIG_VARIANTS {
        let object_key = public_config_object_key(&config.credential.config_key, platform);
        let file = paths.rendered_config_path(platform);
        progress(format!(
            "publish config:{platform}: uploading config object"
        ));
        r2.put_object(
            &config.cloudflare.account_id,
            &config.cloudflare.token,
            &config.cloudflare.r2_bucket,
            &object_key,
            &file,
            config_content_type(platform)?,
        )?;
        progress(format!("publish config:{platform}: uploaded config object"));
    }
    Ok(())
}

fn validate_public_configs(
    paths: &HomePaths,
    config: &Config,
    sing_box: &dyn LocalSingBox,
    mihomo: &dyn LocalMihomo,
    fetcher: &dyn PublicConfigFetcher,
) -> YaoeResult<()> {
    let mut pending: Vec<&'static str> = CONFIG_VARIANTS.to_vec();
    for attempt in 0..CLOUDFLARE_PUBLIC_FETCH_ATTEMPTS {
        let attempt_number = attempt + 1;
        progress(format!(
            "publish config: public fetch attempt {attempt_number}/{} for {} pending platform(s)",
            CLOUDFLARE_PUBLIC_FETCH_ATTEMPTS,
            pending.len()
        ));
        let mut still_pending = Vec::new();
        for platform in pending {
            let url = public_config_url(
                &config.cloudflare.delivery_domain,
                &config.credential.config_key,
                platform,
            );
            match fetcher.fetch_ok(&url)? {
                Some(bytes) => {
                    let mut tmp = tempfile::Builder::new()
                        .prefix(&format!(".public-{platform}."))
                        .suffix(if platform == "clash-verge" {
                            ".yaml"
                        } else {
                            ".json"
                        })
                        .tempfile_in(paths.rendered_config_dir())
                        .map_err(|e| {
                            YaoeError::State(format!(
                                "create temporary public config for {platform}: {e}"
                            ))
                        })?;
                    tmp.write_all(&bytes).map_err(|e| {
                        YaoeError::State(format!("write temporary public config: {e}"))
                    })?;
                    tmp.as_file_mut().sync_all().map_err(|e| {
                        YaoeError::State(format!("fsync temporary public config: {e}"))
                    })?;
                    validate_rendered_config(platform, tmp.path(), sing_box, mihomo)?;
                    log_event(
                        "publish",
                        "config",
                        &[
                            ("platform", platform.to_string()),
                            ("public_fetch", "ok".to_string()),
                        ],
                    );
                }
                None => still_pending.push(platform),
            }
        }
        if still_pending.is_empty() {
            return Ok(());
        }
        pending = still_pending;
        if attempt_number != CLOUDFLARE_PUBLIC_FETCH_ATTEMPTS {
            thread::sleep(Duration::from_secs(
                CLOUDFLARE_PUBLIC_FETCH_INTERVAL_SECONDS,
            ));
        }
    }
    Err(YaoeError::Cloudflare(format!(
        "public config was not available after retries for platform(s): {}",
        pending.join(", ")
    )))
}

fn render_client_entrypoints(parts: &ClientEntrypointParts) -> String {
    let config_url =
        |variant: &str| public_config_url(&parts.delivery_domain, &parts.config_key, variant);
    let gui_profile_url = config_url("clash-verge");
    let import_url = format!(
        "clash://install-config?url={}",
        percent_encode_url(&gui_profile_url)
    );
    let raw_url = |kind: &str, target: &str, ext: &str| {
        format!(
            "https://gitee.com/{}/{}/raw/{}/{}/{}.{}",
            parts.gitee_owner, parts.gitee_repo, GITEE_BOOTSTRAP_BRANCH, kind, target, ext
        )
    };
    format!(
        "clash-verge remote-profile\n{}\n\n\
clash-verge import-url\n{}\n\n\
ios remote-profile\n{}\n\n\
android remote-profile\n{}\n\n\
linux sing-box install\nexport YAOE_CONFIG_KEY='{}'\n\
curl -fsSL {} \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" bash\n\n\
linux sing-box update\nexport YAOE_CONFIG_KEY='{}'\n\
curl -fsSL {} \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" bash\n\n\
macos sing-box install\nexport YAOE_CONFIG_KEY='{}'\n\
curl -fsSL {} \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" /bin/bash\n\n\
macos sing-box update\nexport YAOE_CONFIG_KEY='{}'\n\
curl -fsSL {} \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" /bin/bash\n",
        gui_profile_url,
        import_url,
        config_url("ios"),
        config_url("android"),
        parts.config_key,
        raw_url("install", "linux", "sh"),
        parts.config_key,
        raw_url("update", "linux", "sh"),
        parts.config_key,
        raw_url("install", "macos", "sh"),
        parts.config_key,
        raw_url("update", "macos", "sh"),
    )
}

fn percent_encode_url(url: &str) -> String {
    let mut out = String::new();
    for byte in url.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn selected_server_names(config: &Config, server: Option<&str>) -> YaoeResult<Vec<String>> {
    let mut servers: Vec<String> = config.server.keys().cloned().collect();
    if let Some(one) = server {
        if !config.server.contains_key(one) {
            return Err(YaoeError::Config(format!("unknown server: {one}")));
        }
        servers.retain(|s| s == one);
    }
    Ok(servers)
}

fn server_ssh_key(config: &Config, override_key: Option<&str>) -> YaoeResult<String> {
    override_key
        .map(ToString::to_string)
        .or_else(|| config.ssh.as_ref().map(|s| s.key.clone()))
        .ok_or_else(|| YaoeError::Config("missing SSH key".into()))
}

fn require_remote_active(
    deps: &RuntimeDeps,
    destination: &str,
    key: &str,
    name: &str,
) -> YaoeResult<()> {
    let command = format!("systemctl is-active yaoe-{name}.service");
    let mut last_state = String::new();
    let mut last_stderr = String::new();
    let mut last_error = None;
    for attempt in 1..=10 {
        match deps.ssh.run_as_root_raw(destination, &command, key) {
            Ok(active) => {
                last_error = None;
                last_state = active.stdout.trim().to_string();
                last_stderr = active.stderr.trim().to_string();
                if active.status == 0 && last_state == "active" {
                    return Ok(());
                }
                info(
                    &format!("apply:{name}"),
                    "service active check returned",
                    &[
                        ("attempt", format!("{attempt}/10")),
                        (
                            "state",
                            if last_state.is_empty() {
                                "<empty>".to_string()
                            } else {
                                last_state.clone()
                            },
                        ),
                    ],
                );
            }
            Err(err) => {
                let err_text = err.to_string();
                warn(
                    &format!("apply:{name}"),
                    "service active check failed",
                    &[
                        ("attempt", format!("{attempt}/10")),
                        ("error", one_line(&err_text)),
                    ],
                );
                last_error = Some(err_text);
            }
        }
        if attempt != 10 {
            thread::sleep(Duration::from_secs(2));
        }
    }
    Err(YaoeError::Installer(format!(
        "yaoe-{name}.service is not active after retries; last state: {}; last stderr: {}; last error: {}",
        if last_state.is_empty() {
            "<empty>"
        } else {
            &last_state
        },
        last_stderr,
        last_error.as_deref().unwrap_or("<none>")
    )))
}

fn gitee_delivery(config: &Config) -> GiteeDelivery {
    GiteeDelivery {
        owner: config.gitee.owner.clone(),
        repo: config.gitee.repo.clone(),
        token: config.gitee.token.clone(),
    }
}
