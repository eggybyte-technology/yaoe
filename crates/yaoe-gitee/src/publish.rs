use std::fs;
use std::path::PathBuf;

use yaoe_home::{YaoeError, YaoeResult, atomic_write};

use crate::api::GiteeApi;
use crate::api::Release;
pub use crate::git::BootstrapFile;
use crate::git::GitPublisher;

#[derive(Debug, Clone)]
pub struct GiteeDelivery {
    pub owner: String,
    pub repo: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseAssetStatus {
    LocalMarkerSkip,
    RemoteExists,
    Uploaded,
}

pub fn publish_bootstrap_files(
    paths: &yaoe_home::HomePaths,
    delivery: &GiteeDelivery,
    api: &dyn GiteeApi,
    git: &dyn GitPublisher,
    files: &[BootstrapFile],
) -> YaoeResult<()> {
    let login = api.authenticated_login()?;
    git.publish_bootstrap_files(
        paths,
        &delivery.owner,
        &delivery.repo,
        &login,
        &delivery.token,
        files,
    )
}

pub fn ensure_repository(delivery: &GiteeDelivery, api: &dyn GiteeApi) -> YaoeResult<()> {
    api.ensure_repository(&delivery.owner, &delivery.repo)
}

pub fn ensure_bootstrap_branch(
    paths: &yaoe_home::HomePaths,
    delivery: &GiteeDelivery,
    api: &dyn GiteeApi,
    git: &dyn GitPublisher,
    files: &[BootstrapFile],
) -> YaoeResult<()> {
    let login = api.authenticated_login()?;
    git.ensure_branch_baseline(
        paths,
        &delivery.owner,
        &delivery.repo,
        &login,
        &delivery.token,
        files,
    )
}

pub fn publish_release_assets(
    paths: &yaoe_home::HomePaths,
    delivery: &GiteeDelivery,
    api: &dyn GiteeApi,
    release: &Release,
    assets: &[ReleaseAsset],
) -> YaoeResult<Vec<(String, ReleaseAssetStatus)>> {
    let mut statuses = Vec::new();
    for asset in assets {
        let marker = paths.gitee_release_marker(&asset.name);
        if marker.exists() {
            statuses.push((asset.name.clone(), ReleaseAssetStatus::LocalMarkerSkip));
            continue;
        }
        let remote_names = api.release_asset_names(&delivery.owner, &delivery.repo, release.id)?;
        if remote_names.iter().any(|name| name == &asset.name) {
            atomic_write(&marker, b"ok\n", 0o644)?;
            statuses.push((asset.name.clone(), ReleaseAssetStatus::RemoteExists));
            continue;
        }
        if !fs::metadata(&asset.path).is_ok_and(|m| m.is_file() && m.len() > 0) {
            return Err(YaoeError::Gitee(format!(
                "release asset {} is missing or empty at {}",
                asset.name,
                asset.path.display()
            )));
        }
        api.upload_release_asset(&delivery.owner, &delivery.repo, release.id, &asset.path)?;
        atomic_write(&marker, b"ok\n", 0o644)?;
        statuses.push((asset.name.clone(), ReleaseAssetStatus::Uploaded));
    }
    Ok(statuses)
}

pub fn ensure_release(delivery: &GiteeDelivery, api: &dyn GiteeApi) -> YaoeResult<Release> {
    api.ensure_release(&delivery.owner, &delivery.repo)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeApi {
        remote_assets: Vec<String>,
        uploaded: Mutex<Vec<String>>,
        release_lookups: Mutex<usize>,
    }

    impl GiteeApi for FakeApi {
        fn authenticated_login(&self) -> YaoeResult<String> {
            Ok("owner".to_string())
        }

        fn ensure_repository(&self, _: &str, _: &str) -> YaoeResult<()> {
            Ok(())
        }

        fn ensure_release(&self, _: &str, _: &str) -> YaoeResult<Release> {
            Ok(Release { id: 1 })
        }

        fn release_asset_names(&self, _: &str, _: &str, _: u64) -> YaoeResult<Vec<String>> {
            *self.release_lookups.lock().unwrap() += 1;
            Ok(self.remote_assets.clone())
        }

        fn upload_release_asset(
            &self,
            _: &str,
            _: &str,
            _: u64,
            file: &std::path::Path,
        ) -> YaoeResult<()> {
            self.uploaded.lock().unwrap().push(
                file.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            );
            Ok(())
        }
    }

    fn delivery() -> GiteeDelivery {
        GiteeDelivery {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            token: "token".to_string(),
        }
    }

    #[test]
    fn release_publication_skips_local_ok_marker_without_remote_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(dir.path().join(".yaoe"));
        let asset_path = dir.path().join("asset.bin");
        fs::write(&asset_path, b"asset").unwrap();
        atomic_write(&paths.gitee_release_marker("asset.bin"), b"ok\n", 0o644).unwrap();
        let api = FakeApi::default();

        let statuses = publish_release_assets(
            &paths,
            &delivery(),
            &api,
            &Release { id: 1 },
            &[ReleaseAsset {
                name: "asset.bin".to_string(),
                path: asset_path,
            }],
        )
        .unwrap();

        assert_eq!(
            statuses,
            vec![("asset.bin".to_string(), ReleaseAssetStatus::LocalMarkerSkip)]
        );
        assert_eq!(*api.release_lookups.lock().unwrap(), 0);
        assert!(api.uploaded.lock().unwrap().is_empty());
    }

    #[test]
    fn release_publication_marks_remote_existing_asset_without_upload() {
        let dir = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(dir.path().join(".yaoe"));
        let asset_path = dir.path().join("asset.bin");
        fs::write(&asset_path, b"asset").unwrap();
        let api = FakeApi {
            remote_assets: vec!["asset.bin".to_string()],
            ..FakeApi::default()
        };

        let statuses = publish_release_assets(
            &paths,
            &delivery(),
            &api,
            &Release { id: 1 },
            &[ReleaseAsset {
                name: "asset.bin".to_string(),
                path: asset_path,
            }],
        )
        .unwrap();

        assert_eq!(
            statuses,
            vec![("asset.bin".to_string(), ReleaseAssetStatus::RemoteExists)]
        );
        assert_eq!(
            fs::read_to_string(paths.gitee_release_marker("asset.bin")).unwrap(),
            "ok\n"
        );
        assert!(api.uploaded.lock().unwrap().is_empty());
    }

    #[test]
    fn release_publication_uploads_missing_nonempty_asset_and_marks_it() {
        let dir = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(dir.path().join(".yaoe"));
        let asset_path = dir.path().join("asset.bin");
        fs::write(&asset_path, b"asset").unwrap();
        let api = FakeApi::default();

        let statuses = publish_release_assets(
            &paths,
            &delivery(),
            &api,
            &Release { id: 1 },
            &[ReleaseAsset {
                name: "asset.bin".to_string(),
                path: asset_path,
            }],
        )
        .unwrap();

        assert_eq!(
            statuses,
            vec![("asset.bin".to_string(), ReleaseAssetStatus::Uploaded)]
        );
        assert_eq!(api.uploaded.lock().unwrap().as_slice(), ["asset.bin"]);
        assert_eq!(
            fs::read_to_string(paths.gitee_release_marker("asset.bin")).unwrap(),
            "ok\n"
        );
    }
}
