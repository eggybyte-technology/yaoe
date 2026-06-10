use std::path::{Path, PathBuf};

pub const DEFAULT_HOME: &str = ".yaoe";

#[derive(Debug, Clone)]
pub struct HomePaths {
    pub root: PathBuf,
    pub config: PathBuf,
    pub cache_dir: PathBuf,
    pub acceptance_dir: PathBuf,
    pub work_dir: PathBuf,
}

impl HomePaths {
    pub fn new(home: impl AsRef<Path>) -> Self {
        let root = home.as_ref().to_path_buf();
        Self {
            config: root.join("yaoe.toml"),
            cache_dir: root.join("cache"),
            acceptance_dir: root.join("work/acceptance"),
            work_dir: root.join("work"),
            root,
        }
    }

    pub fn server_package_dir(&self, server: &str) -> PathBuf {
        self.work_dir.join("packages").join(server)
    }

    pub fn server_package_staging_dir(&self, server: &str) -> PathBuf {
        self.server_package_dir(server).join("yaoe-server-package")
    }

    pub fn server_package_archive(&self, server: &str) -> PathBuf {
        self.work_dir
            .join("packages")
            .join(format!("yaoe-server-{server}.tar.gz"))
    }

    pub fn health_probe_path(&self, server: &str) -> PathBuf {
        self.work_dir.join("health").join(server).join("probe.json")
    }

    pub fn upstream_sing_box_archive(&self, variant: &str) -> PathBuf {
        self.cache_dir
            .join("upstream")
            .join(crate::SING_BOX_ARTIFACT_ROOT)
            .join(variant)
            .join(crate::release_asset_name(variant).unwrap_or("unknown"))
    }

    pub fn upstream_srs(&self, asset_name: &str) -> PathBuf {
        self.cache_dir.join("upstream/srs").join(asset_name)
    }

    pub fn server_runtime_sing_box(&self, variant: &str) -> PathBuf {
        self.cache_dir
            .join("server-runtime")
            .join(crate::SING_BOX_ARTIFACT_ROOT)
            .join(variant)
            .join("sing-box")
    }

    pub fn gitee_worktree(&self, owner: &str, repo: &str) -> PathBuf {
        self.cache_dir
            .join("gitee-work")
            .join(owner)
            .join(repo)
            .join(crate::GITEE_BOOTSTRAP_BRANCH)
    }

    pub fn gitee_release_marker(&self, asset_name: &str) -> PathBuf {
        self.cache_dir
            .join("published/gitee-release")
            .join(crate::GITEE_RELEASE_TAG)
            .join(format!("{asset_name}.ok"))
    }

    pub fn gitee_repo_last(&self, path: &str) -> PathBuf {
        self.cache_dir
            .join("published/gitee-repo")
            .join(crate::GITEE_BOOTSTRAP_BRANCH)
            .join(format!("{path}.last"))
    }

    pub fn delivery_repo_work_dir(&self) -> PathBuf {
        self.work_dir.join("delivery/gitee-repo")
    }

    pub fn rendered_config_dir(&self) -> PathBuf {
        self.work_dir.join("delivery/rendered-config")
    }

    pub fn rendered_config_path(&self, variant: &str) -> PathBuf {
        let file = crate::config_variant(variant)
            .map(|entry| entry.public_config_file)
            .unwrap_or("unknown");
        self.rendered_config_dir().join(file)
    }

    pub fn bootstrap_script_path(&self, kind: &str, target: &str) -> PathBuf {
        let ext = crate::script_extension(target).unwrap_or("unknown");
        self.delivery_repo_work_dir()
            .join(kind)
            .join(format!("{target}.{ext}"))
    }

    pub fn gitee_askpass_dir(&self, nonce: &str) -> PathBuf {
        self.work_dir.join("gitee-askpass").join(nonce)
    }
}
