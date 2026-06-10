use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{Builder, Header};
use yaoe_home::{HomePaths, atomic_rename};
use yaoe_home::{YaoeError, YaoeResult, digest_prefix, sha256_hex};
use yaoe_server_installer::{render_install_sh, render_systemd_unit};

use crate::digest::package_input_digest;

#[derive(Debug, Clone)]
pub struct PackageBuildInput {
    pub server_name: String,
    pub runtime_variant: String,
    pub config_json: String,
    pub config_sha256: String,
    pub sing_box_bytes: Vec<u8>,
    pub sing_box_sha256: String,
}

#[derive(Debug, Clone)]
pub struct PackageOutput {
    pub path: PathBuf,
    pub package_sha256: String,
    pub package_input_digest: String,
}

pub fn build_server_package(
    paths: &HomePaths,
    input: &PackageBuildInput,
) -> YaoeResult<PackageOutput> {
    let input_digest = package_input_digest(input);
    let prefix = digest_prefix(&input_digest);
    let package_dir = paths.server_package_dir(&input.server_name);
    if package_dir.exists() {
        fs::remove_dir_all(&package_dir).map_err(|e| YaoeError::State(e.to_string()))?;
    }
    fs::create_dir_all(&package_dir).map_err(|e| YaoeError::State(e.to_string()))?;
    let mut out_dir_guard = PackageDirGuard::new(package_dir.clone());
    if let Some(parent) = package_dir.parent() {
        fs::create_dir_all(parent).map_err(|e| YaoeError::State(e.to_string()))?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|e| YaoeError::State(e.to_string()))?;
    }
    fs::set_permissions(&package_dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| YaoeError::State(e.to_string()))?;
    let out_path = paths.server_package_archive(&input.server_name);
    let tmp_out_path = out_path.with_file_name(format!(
        ".yaoe-server-{}.tar.gz.tmp.{prefix}",
        input.server_name
    ));

    let root = paths.server_package_staging_dir(&input.server_name);
    fs::create_dir_all(root.join("payload/bin")).map_err(|e| YaoeError::State(e.to_string()))?;
    fs::create_dir_all(root.join("payload/config")).map_err(|e| YaoeError::State(e.to_string()))?;
    fs::create_dir_all(root.join("payload/systemd"))
        .map_err(|e| YaoeError::State(e.to_string()))?;

    fs::write(root.join("payload/bin/sing-box"), &input.sing_box_bytes)
        .map_err(|e| YaoeError::State(e.to_string()))?;
    fs::write(
        root.join(format!("payload/config/{}.json", input.server_name)),
        &input.config_json,
    )
    .map_err(|e| YaoeError::State(e.to_string()))?;
    fs::write(
        root.join(format!(
            "payload/systemd/yaoe-{}.service",
            input.server_name
        )),
        render_systemd_unit(&input.server_name),
    )
    .map_err(|e| YaoeError::State(e.to_string()))?;
    fs::write(
        root.join("install.sh"),
        render_install_sh(&input.server_name, &input.runtime_variant)?,
    )
    .map_err(|e| YaoeError::State(e.to_string()))?;

    let tarball = build_deterministic_tar(&package_dir)?;
    let gz_buf = {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&tarball)
            .map_err(|e| YaoeError::State(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| YaoeError::State(e.to_string()))?
    };
    fs::write(&tmp_out_path, &gz_buf).map_err(|e| YaoeError::State(e.to_string()))?;
    atomic_rename(&tmp_out_path, &out_path)?;
    let package_sha256 = sha256_hex(&gz_buf);
    out_dir_guard.keep();

    Ok(PackageOutput {
        path: out_path,
        package_sha256,
        package_input_digest: input_digest,
    })
}

struct PackageDirGuard {
    path: PathBuf,
    keep: bool,
}

impl PackageDirGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for PackageDirGuard {
    fn drop(&mut self) {
        if !self.keep && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn build_deterministic_tar(root: &Path) -> YaoeResult<Vec<u8>> {
    let mut entries: Vec<(PathBuf, PathBuf)> = Vec::new();
    collect_files(root, root, &mut entries)?;
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut tar_buf = Vec::new();
    {
        let mut builder = Builder::new(&mut tar_buf);
        for (rel, abs) in &entries {
            let mut header = Header::new_gnu();
            let data = fs::read(abs).map_err(|e| YaoeError::State(e.to_string()))?;
            let mode = if rel.to_string_lossy().contains("sing-box")
                || rel.extension().is_some_and(|e| e == "sh")
            {
                0o755
            } else if rel.extension().map(|e| e == "json").unwrap_or(false) {
                0o600
            } else {
                0o644
            };
            header.set_size(data.len() as u64);
            header.set_mode(mode);
            header.set_mtime(0);
            header.set_uid(0);
            header.set_gid(0);
            header
                .set_username("root")
                .map_err(|e| YaoeError::State(e.to_string()))?;
            header
                .set_groupname("root")
                .map_err(|e| YaoeError::State(e.to_string()))?;
            header
                .set_path(rel.to_string_lossy().as_ref())
                .map_err(|e| YaoeError::State(e.to_string()))?;
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            builder
                .append(&header, &data[..])
                .map_err(|e| YaoeError::State(e.to_string()))?;
        }
        builder
            .finish()
            .map_err(|e| YaoeError::State(e.to_string()))?;
    }

    Ok(tar_buf)
}

fn collect_files(base: &Path, dir: &Path, out: &mut Vec<(PathBuf, PathBuf)>) -> YaoeResult<()> {
    for entry in fs::read_dir(dir).map_err(|e| YaoeError::State(e.to_string()))? {
        let entry = entry.map_err(|e| YaoeError::State(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| YaoeError::State(e.to_string()))?;
            out.push((rel.to_path_buf(), path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::collections::BTreeMap;
    use std::io::Read;
    use yaoe_home::HomePaths;

    fn input() -> PackageBuildInput {
        PackageBuildInput {
            server_name: "hk".into(),
            runtime_variant: "linux-amd64".into(),
            config_json: r#"{"log":{}}"#.into(),
            config_sha256: sha256_hex(b"cfg"),
            sing_box_bytes: vec![0x7f; 64],
            sing_box_sha256: sha256_hex(&[0x7f; 64]),
        }
    }

    #[test]
    fn package_is_deterministic() {
        let paths = HomePaths::new(tempfile::tempdir().unwrap().path().join(".yaoe"));
        let a = build_server_package(&paths, &input()).unwrap();
        let b = build_server_package(&paths, &input()).unwrap();
        let bytes_a = fs::read(&a.path).unwrap();
        let bytes_b = fs::read(&b.path).unwrap();
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn package_contains_only_v0_0_1_payload() {
        let paths = HomePaths::new(tempfile::tempdir().unwrap().path().join(".yaoe"));
        let pkg = build_server_package(&paths, &input()).unwrap();
        let bytes = fs::read(&pkg.path).unwrap();
        let decoder = GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(decoder);
        let mut modes = BTreeMap::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().to_string();
            modes.insert(path, entry.header().mode().unwrap());
            let mut sink = Vec::new();
            entry.read_to_end(&mut sink).unwrap();
        }
        let paths: Vec<_> = modes.keys().cloned().collect();
        assert_eq!(
            paths,
            vec![
                "yaoe-server-package/install.sh",
                "yaoe-server-package/payload/bin/sing-box",
                "yaoe-server-package/payload/config/hk.json",
                "yaoe-server-package/payload/systemd/yaoe-hk.service",
            ]
        );
        assert!(!paths.iter().any(|p| p.contains("cert")));
    }
}
