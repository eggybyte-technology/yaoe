use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use flate2::read::GzDecoder;
use yaoe_home::{
    GITEE_RELEASE_TAG, YaoeError, YaoeResult, atomic_write, is_managed_server_runtime_variant,
    release_asset_name, sha256_hex, sing_box_version_line, upstream_sing_box_url,
};

use crate::fetch::{HttpFetcher, RuntimeAssetKind, fetch_to_cache, non_empty_file};

#[derive(Debug, Clone)]
pub struct ResolvedServerRuntime {
    pub path: std::path::PathBuf,
    pub sha256: String,
}

pub fn resolve_server_runtime(
    paths: &yaoe_home::HomePaths,
    variant: &str,
    owner: &str,
    repo: &str,
    fetcher: &dyn HttpFetcher,
) -> YaoeResult<ResolvedServerRuntime> {
    if !is_managed_server_runtime_variant(variant) {
        return Err(YaoeError::Internal(format!(
            "unsupported managed server runtime variant: {variant}"
        )));
    }
    let exe = paths.server_runtime_sing_box(variant);
    if valid_sing_box(&exe) {
        return resolved(exe);
    }

    let archive = paths.upstream_sing_box_archive(variant);
    if non_empty_file(&archive) && extract_runtime(&archive, &exe).is_ok() && valid_sing_box(&exe) {
        return resolved(exe);
    }

    let asset_name = release_asset_name(variant)
        .ok_or_else(|| YaoeError::Internal("missing managed server asset".into()))?;
    let gitee_url = format!(
        "https://gitee.com/{owner}/{repo}/releases/download/{GITEE_RELEASE_TAG}/{asset_name}"
    );
    if let Ok(bytes) = fetcher.fetch(&gitee_url)
        && !bytes.is_empty()
    {
        fetch_to_cache(&archive, &bytes, RuntimeAssetKind::SingBox)?;
        if extract_runtime(&archive, &exe).is_ok() && valid_sing_box(&exe) {
            return resolved(exe);
        }
    }

    let upstream_url = upstream_sing_box_url(variant)
        .ok_or_else(|| YaoeError::Internal("missing managed server upstream URL".into()))?;
    let bytes = fetcher
        .fetch(&upstream_url)
        .map_err(|e| YaoeError::Cache(format!("server runtime resolution failed: {e}")))?;
    fetch_to_cache(&archive, &bytes, RuntimeAssetKind::SingBox)?;
    extract_runtime(&archive, &exe)?;
    if valid_sing_box(&exe) {
        return resolved(exe);
    }
    Err(YaoeError::Cache(
        "required server runtime cache entry missing or invalid after resolution".into(),
    ))
}

fn extract_runtime(archive: &Path, dest: &Path) -> YaoeResult<()> {
    let file = fs::File::open(archive)
        .map_err(|e| YaoeError::Cache(format!("open {}: {e}", archive.display())))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive
        .entries()
        .map_err(|e| YaoeError::Cache(format!("read archive: {e}")))?
    {
        let mut entry = entry.map_err(|e| YaoeError::Cache(format!("archive entry: {e}")))?;
        let path = entry
            .path()
            .map_err(|e| YaoeError::Cache(format!("archive path: {e}")))?
            .to_path_buf();
        if path.file_name().and_then(|s| s.to_str()) == Some("sing-box") {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|e| YaoeError::Cache(format!("read sing-box from archive: {e}")))?;
            atomic_write(dest, &bytes, 0o755)?;
            return Ok(());
        }
    }
    Err(YaoeError::Cache(
        "sing-box executable not found in server runtime archive".into(),
    ))
}

fn valid_sing_box(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
        return false;
    }
    let Ok(output) = Command::new(path).arg("version").output() else {
        return false;
    };
    let first_line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    output.status.success() && first_line == sing_box_version_line()
}

fn resolved(path: std::path::PathBuf) -> YaoeResult<ResolvedServerRuntime> {
    let bytes =
        fs::read(&path).map_err(|e| YaoeError::Cache(format!("read {}: {e}", path.display())))?;
    Ok(ResolvedServerRuntime {
        path,
        sha256: sha256_hex(&bytes),
    })
}
