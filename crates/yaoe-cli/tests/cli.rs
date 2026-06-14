use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;

fn valid_config() -> String {
    format!(
        r#"[ssh]
key = "~/.ssh/id_ed25519"

[cloudflare]
token = "secret-token-that-does-not-match-cf-regex"
account_id = "account123"
delivery_domain = "cfg.test.net"
r2_bucket = "yaoe-config"

[gitee]
token = "gitee-secret-token"
owner = "owner"
repo = "repo"

[credential]
vless_uuid = "550e8400-e29b-41d4-a716-446655440000"
config_key = "{}"
reality_private_key = "sBFVzOLPGBcFMR55fkt7_5xY5SGVD7Vsw8jrLEObh1I"
reality_short_id = "0123456789abcdef"

[reality]
handshake_server = "www.cloudflare.com"

[server.hk]
ssh = "root@198.51.100.10"
ip = "198.51.100.10"
port = 28443
"#,
        "A".repeat(128)
    )
}

#[test]
fn rejects_unknown_command_with_code_2() {
    Command::cargo_bin("yaoe")
        .unwrap()
        .arg("unknown-command")
        .assert()
        .code(2);
}

#[test]
fn rejects_removed_commands_with_code_2() {
    for args in [
        vec!["singbox"],
        vec!["logs", "hk"],
        vec!["restart", "hk"],
        vec!["publish", "dns"],
        vec!["publish", "srs"],
        vec!["get", "config", "ios"],
        vec!["rotate", "credential"],
        vec!["rotate", "config-key", "--yes"],
    ] {
        Command::cargo_bin("yaoe")
            .unwrap()
            .args(args)
            .assert()
            .code(2);
    }
}

#[test]
fn client_prints_exact_entrypoint_block() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir(&home).unwrap();
    let key = "A".repeat(128);
    fs::write(
        home.join("yaoe.toml"),
        format!(
            r#"[cloudflare]
delivery_domain = "cfg.test.net"

[gitee]
owner = "owner"
repo = "repo"

[credential]
config_key = "{key}"
"#
        ),
    )
    .unwrap();

    Command::cargo_bin("yaoe")
        .unwrap()
        .current_dir(dir.path())
        .arg("client")
        .assert()
        .success()
        .stdout(format!(
            "clash-verge remote-profile\nhttps://cfg.test.net/config/{key}/clash-verge.yaml\n\n\
clash-verge import-url\nclash://install-config?url=https%3A%2F%2Fcfg.test.net%2Fconfig%2F{key}%2Fclash-verge.yaml\n\n\
ios remote-profile\nhttps://cfg.test.net/config/{key}/ios.json\n\n\
android remote-profile\nhttps://cfg.test.net/config/{key}/android.json\n\n\
linux sing-box install\nexport YAOE_CONFIG_KEY='{key}'\n\
curl -fsSL https://gitee.com/owner/repo/raw/main/install/linux.sh \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" bash\n\n\
linux sing-box update\nexport YAOE_CONFIG_KEY='{key}'\n\
curl -fsSL https://gitee.com/owner/repo/raw/main/update/linux.sh \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" bash\n\n\
macos sing-box install\nexport YAOE_CONFIG_KEY='{key}'\n\
curl -fsSL https://gitee.com/owner/repo/raw/main/install/macos.sh \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" /bin/bash\n\n\
macos sing-box update\nexport YAOE_CONFIG_KEY='{key}'\n\
curl -fsSL https://gitee.com/owner/repo/raw/main/update/macos.sh \\\n  | sudo env YAOE_CONFIG_KEY=\"$YAOE_CONFIG_KEY\" /bin/bash\n"
        ))
        .stderr("");
}

#[test]
fn client_requires_gitee_coordinates_with_code_3() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir(&home).unwrap();
    fs::write(
        home.join("yaoe.toml"),
        format!(
            r#"[cloudflare]
delivery_domain = "cfg.test.net"

[credential]
config_key = "{}"
"#,
            "A".repeat(128)
        ),
    )
    .unwrap();

    Command::cargo_bin("yaoe")
        .unwrap()
        .current_dir(dir.path())
        .arg("client")
        .assert()
        .code(3);
}

#[test]
fn publish_requires_subcommand() {
    Command::cargo_bin("yaoe")
        .unwrap()
        .arg("publish")
        .assert()
        .code(2);
}

#[test]
fn mutating_command_reports_invalid_config_before_home_side_effects() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir(&home).unwrap();
    fs::write(home.join("yaoe.toml"), "not valid toml =").unwrap();

    Command::cargo_bin("yaoe")
        .unwrap()
        .current_dir(dir.path())
        .arg("apply")
        .assert()
        .code(3);
    assert!(!home.join("work").exists());
}

#[test]
fn render_config_prints_exact_rendered_paths() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir(&home).unwrap();
    fs::write(home.join("yaoe.toml"), valid_config()).unwrap();

    Command::cargo_bin("yaoe")
        .unwrap()
        .current_dir(dir.path())
        .args(["render", "config"])
        .assert()
        .success()
        .stdout(
            "render clash-verge .yaoe/work/delivery/rendered-config/clash-verge.yaml\n\
render linux-amd64 .yaoe/work/delivery/rendered-config/linux-amd64.json\n\
render linux-arm64 .yaoe/work/delivery/rendered-config/linux-arm64.json\n\
render macos-amd64 .yaoe/work/delivery/rendered-config/macos-amd64.json\n\
render macos-arm64 .yaoe/work/delivery/rendered-config/macos-arm64.json\n\
render ios .yaoe/work/delivery/rendered-config/ios.json\n\
render android .yaoe/work/delivery/rendered-config/android.json\n",
        );
}

#[test]
fn redacts_provider_tokens_from_errors() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir(&home).unwrap();
    let config = valid_config().replace(
        "ssh = \"root@198.51.100.10\"",
        "ssh = \"root@secret-token-that-does-not-match-cf-regex\"",
    ) + "\n[server.jp]\nssh = \"root@secret-token-that-does-not-match-cf-regex\"\nip = \"198.51.100.11\"\nport = 28444\n";
    fs::write(home.join("yaoe.toml"), config).unwrap();

    Command::cargo_bin("yaoe")
        .unwrap()
        .current_dir(dir.path())
        .arg("check")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("<redacted>"))
        .stderr(predicate::str::contains("secret-token-that-does-not-match-cf-regex").not())
        .stderr(predicate::str::contains("gitee-secret-token").not());
}

#[test]
fn rotate_config_key_prints_new_key_only_and_next_step() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir(&home).unwrap();
    let old_key = "A".repeat(128);
    fs::write(home.join("yaoe.toml"), valid_config()).unwrap();

    let assert = Command::cargo_bin("yaoe")
        .unwrap()
        .current_dir(dir.path())
        .args(["rotate", "config-key"])
        .assert()
        .success()
        .stdout(predicate::str::contains("next yaoe publish config"))
        .stdout(predicate::str::contains(&old_key).not());

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let new_key = stdout
        .lines()
        .find_map(|line| line.strip_prefix("config_key "))
        .expect("new config key line");
    assert_eq!(new_key.len(), 128);
    assert_ne!(new_key, old_key);

    let updated = fs::read_to_string(home.join("yaoe.toml")).unwrap();
    assert!(updated.contains(&format!("config_key = \"{new_key}\"")));
    assert!(!updated.contains(&format!("config_key = \"{old_key}\"")));
}

#[test]
fn rotate_vless_uuid_prints_apply_and_publish_steps_without_old_uuid() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join(".yaoe");
    fs::create_dir(&home).unwrap();
    let old_uuid = "550e8400-e29b-41d4-a716-446655440000";
    fs::write(home.join("yaoe.toml"), valid_config()).unwrap();

    let assert = Command::cargo_bin("yaoe")
        .unwrap()
        .current_dir(dir.path())
        .args(["rotate", "vless-uuid"])
        .assert()
        .success()
        .stdout(predicate::str::contains("next yaoe apply"))
        .stdout(predicate::str::contains("next yaoe publish config"))
        .stdout(predicate::str::contains(old_uuid).not());

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let new_uuid = stdout
        .lines()
        .find_map(|line| line.strip_prefix("vless_uuid "))
        .expect("new vless uuid line");
    assert_ne!(new_uuid, old_uuid);

    let updated = fs::read_to_string(home.join("yaoe.toml")).unwrap();
    assert!(updated.contains(&format!("vless_uuid = \"{new_uuid}\"")));
    assert!(!updated.contains(&format!("vless_uuid = \"{old_uuid}\"")));
}
