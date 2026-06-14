use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::{YaoeError, YaoeResult};

use crate::paths::{DEFAULT_HOME, HomePaths};

pub fn resolve_home(home: Option<&Path>) -> crate::paths::HomePaths {
    HomePaths::new(home.unwrap_or_else(|| Path::new(DEFAULT_HOME)))
}

pub fn init_home(paths: &HomePaths) -> YaoeResult<()> {
    if paths.root.exists() && !paths.root.is_dir() {
        return Err(YaoeError::State(format!(
            "{} exists and is not a directory",
            paths.root.display()
        )));
    }
    if !paths.root.exists() {
        fs::create_dir_all(&paths.root)
            .map_err(|e| YaoeError::State(format!("mkdir {}: {e}", paths.root.display())))?;
        set_mode(&paths.root, 0o700)?;
    }
    ensure_home_dirs(paths)?;
    Ok(())
}

pub fn ensure_home(paths: &HomePaths) -> YaoeResult<()> {
    if !paths.root.is_dir() {
        return Err(YaoeError::State(format!(
            "{} is not an initialized YAOE home",
            paths.root.display()
        )));
    }
    ensure_home_dirs(paths)?;
    Ok(())
}

fn ensure_home_dirs(paths: &HomePaths) -> YaoeResult<()> {
    for d in required_home_dirs(paths) {
        fs::create_dir_all(&d)
            .map_err(|e| YaoeError::State(format!("mkdir {}: {e}", d.display())))?;
    }
    set_mode(&paths.work_dir, 0o700)?;
    Ok(())
}

pub fn validate_home_layout(paths: &HomePaths) -> YaoeResult<()> {
    if !paths.root.is_dir() {
        return Err(YaoeError::State(format!(
            "{} is not an initialized YAOE home",
            paths.root.display()
        )));
    }
    for dir in required_home_dirs(paths) {
        if !dir.is_dir() {
            return Err(YaoeError::State(format!(
                "required YAOE directory is missing: {}",
                dir.display()
            )));
        }
    }
    Ok(())
}

fn required_home_dirs(paths: &HomePaths) -> Vec<PathBuf> {
    let mut dirs = vec![
        paths.cache_dir.clone(),
        paths
            .cache_dir
            .join("upstream")
            .join(crate::SING_BOX_ARTIFACT_ROOT),
        paths.cache_dir.join("upstream/srs"),
        paths.cache_dir.join("server-runtime"),
        paths.cache_dir.join("gitee-work"),
        paths
            .cache_dir
            .join("published/gitee-release")
            .join(crate::GITEE_RELEASE_TAG),
        paths
            .cache_dir
            .join("published/gitee-repo")
            .join(crate::GITEE_BOOTSTRAP_BRANCH),
        paths.work_dir.clone(),
        paths.work_dir.join("delivery/gitee-repo/install"),
        paths.work_dir.join("delivery/gitee-repo/update"),
        paths.work_dir.join("delivery/rendered-config"),
        paths.work_dir.join("packages"),
        paths.work_dir.join("health"),
        paths.work_dir.join("gitee-askpass"),
        paths.acceptance_dir.clone(),
    ];
    for variant in crate::SERVICE_CONFIG_VARIANTS {
        dirs.push(
            paths
                .cache_dir
                .join("upstream")
                .join(crate::SING_BOX_ARTIFACT_ROOT)
                .join(variant),
        );
    }
    for variant in crate::MANAGED_SERVER_RUNTIME_VARIANTS {
        dirs.push(
            paths
                .cache_dir
                .join("server-runtime")
                .join(crate::SING_BOX_ARTIFACT_ROOT)
                .join(variant),
        );
    }
    dirs
}

pub fn atomic_write(path: &Path, data: &[u8], mode: u32) -> YaoeResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| YaoeError::State(format!("mkdir {}: {e}", parent.display())))?;
    }
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("atomic");
    let tmp = path.with_file_name(format!(".{file_name}.tmp.{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| YaoeError::State(format!("create {}: {e}", tmp.display())))?;
    file.set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|e| YaoeError::State(format!("chmod {}: {e}", tmp.display())))?;
    file.write_all(data)
        .map_err(|e| YaoeError::State(format!("write {}: {e}", tmp.display())))?;
    file.sync_all()
        .map_err(|e| YaoeError::State(format!("fsync {}: {e}", tmp.display())))?;
    drop(file);
    fs::rename(&tmp, path)
        .map_err(|e| YaoeError::State(format!("rename {}: {e}", path.display())))?;
    if let Some(parent) = path.parent() {
        let dir = OpenOptions::new()
            .read(true)
            .open(parent)
            .map_err(|e| YaoeError::State(format!("open {}: {e}", parent.display())))?;
        dir.sync_all()
            .map_err(|e| YaoeError::State(format!("fsync {}: {e}", parent.display())))?;
    }
    Ok(())
}

pub fn atomic_rename(from: &Path, to: &Path) -> YaoeResult<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| YaoeError::State(format!("mkdir {}: {e}", parent.display())))?;
    }
    fs::rename(from, to).map_err(|e| {
        YaoeError::State(format!(
            "rename {} to {}: {e}",
            from.display(),
            to.display()
        ))
    })?;
    if let Some(parent) = to.parent() {
        let dir = OpenOptions::new()
            .read(true)
            .open(parent)
            .map_err(|e| YaoeError::State(format!("open {}: {e}", parent.display())))?;
        dir.sync_all()
            .map_err(|e| YaoeError::State(format!("fsync {}: {e}", parent.display())))?;
    }
    Ok(())
}

pub fn set_mode(path: &Path, mode: u32) -> YaoeResult<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|e| YaoeError::State(format!("chmod {}: {e}", path.display())))
}

pub fn read_secret_file(path: &Path) -> YaoeResult<String> {
    let data = fs::read_to_string(path)
        .map_err(|e| YaoeError::State(format!("read {}: {e}", path.display())))?;
    Ok(data)
}

pub fn write_secret_file(path: &Path, content: &str) -> YaoeResult<()> {
    atomic_write(path, content.as_bytes(), 0o600)
}

pub fn require_regular_file(path: &Path) -> YaoeResult<()> {
    let meta = fs::metadata(path)
        .map_err(|e| YaoeError::State(format!("metadata {}: {e}", path.display())))?;
    if !meta.is_file() {
        return Err(YaoeError::State(format!(
            "{} must be a regular file",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_fixed_v0_0_1_layout_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HomePaths::new(tmp.path().join(".yaoe"));

        init_home(&paths).unwrap();
        validate_home_layout(&paths).unwrap();

        for dir in [
            paths
                .cache_dir
                .join("upstream/sing-box/1.13.13/linux-amd64"),
            paths
                .cache_dir
                .join("upstream/sing-box/1.13.13/linux-arm64"),
            paths
                .cache_dir
                .join("server-runtime/sing-box/1.13.13/linux-amd64"),
            paths
                .cache_dir
                .join("server-runtime/sing-box/1.13.13/linux-arm64"),
            paths.cache_dir.join("upstream/srs"),
            paths
                .cache_dir
                .join("published/gitee-release/yaoe-v0.0.1-sing-box-1.13.13"),
            paths.cache_dir.join("published/gitee-repo/main"),
            paths.work_dir.join("delivery/gitee-repo/install"),
            paths.work_dir.join("delivery/gitee-repo/update"),
            paths.work_dir.join("delivery/rendered-config"),
            paths.work_dir.join("packages"),
            paths.work_dir.join("health"),
            paths.work_dir.join("acceptance"),
        ] {
            assert!(dir.is_dir(), "missing {}", dir.display());
        }
    }
}
