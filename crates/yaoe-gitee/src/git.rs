use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use yaoe_home::{GITEE_BOOTSTRAP_BRANCH, HomePaths, YaoeError, YaoeResult, atomic_write};

#[derive(Debug, Clone)]
pub struct BootstrapFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub trait GitPublisher: Send + Sync {
    fn ensure_branch_baseline(
        &self,
        paths: &HomePaths,
        owner: &str,
        repo: &str,
        login: &str,
        token: &str,
        files: &[BootstrapFile],
    ) -> YaoeResult<()>;

    fn publish_bootstrap_files(
        &self,
        paths: &HomePaths,
        owner: &str,
        repo: &str,
        login: &str,
        token: &str,
        files: &[BootstrapFile],
    ) -> YaoeResult<()>;
}

pub struct SystemGitPublisher;

impl GitPublisher for SystemGitPublisher {
    fn ensure_branch_baseline(
        &self,
        paths: &HomePaths,
        owner: &str,
        repo: &str,
        login: &str,
        token: &str,
        files: &[BootstrapFile],
    ) -> YaoeResult<()> {
        ensure_branch(paths, owner, repo, login, token, files, false)
    }

    fn publish_bootstrap_files(
        &self,
        paths: &HomePaths,
        owner: &str,
        repo: &str,
        login: &str,
        token: &str,
        files: &[BootstrapFile],
    ) -> YaoeResult<()> {
        ensure_branch(paths, owner, repo, login, token, files, true)
    }
}

fn ensure_branch(
    paths: &HomePaths,
    owner: &str,
    repo: &str,
    login: &str,
    token: &str,
    files: &[BootstrapFile],
    publish_existing_changes: bool,
) -> YaoeResult<()> {
    let worktree = paths.gitee_worktree(owner, repo);
    fs::create_dir_all(&worktree)
        .map_err(|e| YaoeError::Gitee(format!("mkdir {}: {e}", worktree.display())))?;
    let askpass = Askpass::new(paths, login, token)?;
    git(&worktree, &askpass, &["init"])?;
    git(&worktree, &askpass, &["remote", "remove", "origin"]).ok();
    git(
        &worktree,
        &askpass,
        &[
            "remote",
            "add",
            "origin",
            &format!("https://gitee.com/{owner}/{repo}.git"),
        ],
    )?;
    let branch_exists = fetch_branch_exists(&worktree, &askpass)?;
    if branch_exists {
        let remote_branch = format!("origin/{GITEE_BOOTSTRAP_BRANCH}");
        git(
            &worktree,
            &askpass,
            &["checkout", "-B", GITEE_BOOTSTRAP_BRANCH, &remote_branch],
        )?;
    } else {
        git(&worktree, &askpass, &["checkout", "--detach"]).ok();
        git(
            &worktree,
            &askpass,
            &["branch", "-D", GITEE_BOOTSTRAP_BRANCH],
        )
        .ok();
        git(
            &worktree,
            &askpass,
            &["checkout", "--orphan", GITEE_BOOTSTRAP_BRANCH],
        )?;
        git(&worktree, &askpass, &["rm", "-rf", "."]).ok();
    }

    if branch_exists && !publish_existing_changes {
        return Ok(());
    }

    let desired_paths = files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<BTreeSet<_>>();
    let stale_removed = if branch_exists {
        remove_stale_bootstrap_files(paths, &worktree, &desired_paths)?
    } else {
        0
    };

    let mut changed = Vec::new();
    for file in files {
        let last = paths.gitee_repo_last(&file.path);
        if branch_exists && fs::read(&last).is_ok_and(|old| old == file.bytes) {
            continue;
        }
        let target = worktree.join(&file.path);
        if branch_exists && fs::read(&target).is_ok_and(|old| old == file.bytes) {
            atomic_write(&last, &file.bytes, 0o644)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| YaoeError::Gitee(format!("mkdir {}: {e}", parent.display())))?;
        }
        fs::write(&target, &file.bytes)
            .map_err(|e| YaoeError::Gitee(format!("write {}: {e}", target.display())))?;
        changed.push(file.path.clone());
    }

    if !changed.is_empty() || stale_removed > 0 || !branch_exists {
        git(&worktree, &askpass, &["add", "-A", "install", "update"])?;
        let status = git_output(&worktree, &askpass, &["status", "--porcelain"])?;
        if !status.trim().is_empty() {
            git(
                &worktree,
                &askpass,
                &[
                    "-c",
                    "user.name=YAOE",
                    "-c",
                    "user.email=yaoe@example.invalid",
                    "commit",
                    "-m",
                    "yaoe v0.0.1 bootstrap scripts",
                ],
            )?;
            git(
                &worktree,
                &askpass,
                &["push", "-u", "origin", GITEE_BOOTSTRAP_BRANCH],
            )?;
        }
        for file in files {
            atomic_write(&paths.gitee_repo_last(&file.path), &file.bytes, 0o644)?;
        }
    }
    Ok(())
}

fn remove_stale_bootstrap_files(
    paths: &HomePaths,
    worktree: &Path,
    desired_paths: &BTreeSet<&str>,
) -> YaoeResult<usize> {
    let mut removed = 0;
    for stale in stale_bootstrap_paths(worktree, desired_paths)? {
        let path = worktree.join(&stale);
        fs::remove_file(&path)
            .map_err(|e| YaoeError::Gitee(format!("remove stale bootstrap {stale}: {e}")))?;
        removed += 1;
        let marker = paths.gitee_repo_last(&stale);
        match fs::remove_file(&marker) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(YaoeError::Gitee(format!(
                    "remove stale bootstrap marker {}: {err}",
                    marker.display()
                )));
            }
        }
    }
    Ok(removed)
}

fn stale_bootstrap_paths(
    worktree: &Path,
    desired_paths: &BTreeSet<&str>,
) -> YaoeResult<Vec<String>> {
    let mut stale = Vec::new();
    for root in ["install", "update"] {
        let dir = worktree.join(root);
        if !dir.exists() {
            continue;
        }
        collect_stale_bootstrap_paths(worktree, &dir, desired_paths, &mut stale)?;
    }
    stale.sort();
    Ok(stale)
}

fn collect_stale_bootstrap_paths(
    worktree: &Path,
    dir: &Path,
    desired_paths: &BTreeSet<&str>,
    stale: &mut Vec<String>,
) -> YaoeResult<()> {
    for entry in fs::read_dir(dir)
        .map_err(|e| YaoeError::Gitee(format!("read bootstrap dir {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| YaoeError::Gitee(format!("read bootstrap entry: {e}")))?;
        let path = entry.path();
        if path.is_dir() {
            collect_stale_bootstrap_paths(worktree, &path, desired_paths, stale)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(worktree)
            .map_err(|e| YaoeError::Gitee(format!("bootstrap path outside worktree: {e}")))?
            .to_string_lossy()
            .replace('\\', "/");
        if !desired_paths.contains(rel.as_str()) {
            stale.push(rel);
        }
    }
    Ok(())
}

fn fetch_branch_exists(worktree: &Path, askpass: &Askpass) -> YaoeResult<bool> {
    let mut last_error = None;
    for attempt in 1..=3 {
        match git(
            worktree,
            askpass,
            &["fetch", "origin", GITEE_BOOTSTRAP_BRANCH],
        ) {
            Ok(()) => return Ok(true),
            Err(err) => {
                last_error = Some(err.to_string());
                if attempt != 3 {
                    std::thread::sleep(std::time::Duration::from_secs(attempt));
                }
            }
        }
    }
    let remote = git_output(
        worktree,
        askpass,
        &["ls-remote", "--heads", "origin", GITEE_BOOTSTRAP_BRANCH],
    )?;
    if remote.trim().is_empty() {
        Ok(false)
    } else {
        Err(YaoeError::Gitee(format!(
            "fetch origin {GITEE_BOOTSTRAP_BRANCH} failed after retries: {}",
            last_error.as_deref().unwrap_or("<none>")
        )))
    }
}

struct Askpass {
    _dir: TempDir,
    path: PathBuf,
    login: String,
    token: String,
}

impl Askpass {
    fn new(paths: &HomePaths, login: &str, token: &str) -> YaoeResult<Self> {
        let root = paths.work_dir.join("gitee-askpass");
        fs::create_dir_all(&root)
            .map_err(|e| YaoeError::Gitee(format!("mkdir {}: {e}", root.display())))?;
        let dir = tempfile::Builder::new()
            .prefix("askpass-")
            .tempdir_in(root)
            .map_err(|e| YaoeError::Gitee(format!("create askpass dir: {e}")))?;
        let path = dir.path().join("askpass.sh");
        let script = r#"#!/bin/sh
case "$1" in
  *Username*) printf '%s\n' "$YAOE_GITEE_LOGIN" ;;
  *Password*) printf '%s\n' "$YAOE_GITEE_TOKEN" ;;
  *) printf '\n' ;;
esac
"#;
        fs::write(&path, script).map_err(|e| YaoeError::Gitee(format!("write askpass: {e}")))?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
            .map_err(|e| YaoeError::Gitee(format!("chmod askpass: {e}")))?;
        Ok(Self {
            _dir: dir,
            path,
            login: login.to_string(),
            token: token.to_string(),
        })
    }
}

fn git(worktree: &Path, askpass: &Askpass, args: &[&str]) -> YaoeResult<()> {
    git_output(worktree, askpass, args).map(|_| ())
}

fn git_output(worktree: &Path, askpass: &Askpass, args: &[&str]) -> YaoeResult<String> {
    let output = Command::new("git")
        .current_dir(worktree)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", &askpass.path)
        .env("YAOE_GITEE_LOGIN", &askpass.login)
        .env("YAOE_GITEE_TOKEN", &askpass.token)
        .arg("-c")
        .arg("credential.helper=")
        .args(args)
        .output()
        .map_err(|e| YaoeError::Gitee(format!("run git: {e}")))?;
    if !output.status.success() {
        let stderr = sanitize_git_stderr(
            &String::from_utf8_lossy(&output.stderr),
            &askpass.login,
            &askpass.token,
        );
        return Err(YaoeError::Gitee(format!("git failed: {}", stderr.trim())));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn sanitize_git_stderr(stderr: &str, login: &str, token: &str) -> String {
    stderr
        .replace(login, "<redacted>")
        .replace(token, "<redacted>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_architecture_bootstrap_files_are_removed_with_markers() {
        let dir = tempfile::tempdir().unwrap();
        let paths = HomePaths::new(dir.path().join(".yaoe"));
        let worktree = paths.gitee_worktree("owner", "repo");
        atomic_write(&worktree.join("install/linux.sh"), b"keep", 0o644).unwrap();
        atomic_write(&worktree.join("update/macos.sh"), b"keep", 0o644).unwrap();
        atomic_write(&worktree.join("install/linux-amd64.sh"), b"stale", 0o644).unwrap();
        atomic_write(&worktree.join("update/darwin-arm64.sh"), b"stale", 0o644).unwrap();
        atomic_write(
            &paths.gitee_repo_last("install/linux-amd64.sh"),
            b"stale",
            0o644,
        )
        .unwrap();
        atomic_write(
            &paths.gitee_repo_last("update/darwin-arm64.sh"),
            b"stale",
            0o644,
        )
        .unwrap();
        let desired = ["install/linux.sh", "update/macos.sh"]
            .into_iter()
            .collect::<BTreeSet<_>>();

        let removed = remove_stale_bootstrap_files(&paths, &worktree, &desired).unwrap();

        assert_eq!(removed, 2);
        assert!(worktree.join("install/linux.sh").is_file());
        assert!(worktree.join("update/macos.sh").is_file());
        assert!(!worktree.join("install/linux-amd64.sh").exists());
        assert!(!worktree.join("update/darwin-arm64.sh").exists());
        assert!(!paths.gitee_repo_last("install/linux-amd64.sh").exists());
        assert!(!paths.gitee_repo_last("update/darwin-arm64.sh").exists());
    }

    #[test]
    fn git_stderr_redacts_askpass_credentials() {
        let stderr = sanitize_git_stderr(
            "fatal: authentication failed for owner with gitee_token_123",
            "owner",
            "gitee_token_123",
        );

        assert!(!stderr.contains("owner"));
        assert!(!stderr.contains("gitee_token_123"));
        assert_eq!(
            stderr,
            "fatal: authentication failed for <redacted> with <redacted>"
        );
    }
}
