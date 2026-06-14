//! Runtime dependency assembly and command-scoped no-op adapters.

use std::path::Path;

use yaoe_cloudflare::{CloudflareClient, CloudflareZoneResolver, R2Wrangler, SystemR2Wrangler};
use yaoe_config::Config;
use yaoe_gitee::{BootstrapFile, GitPublisher, GiteeHttpApi, SystemGitPublisher};
use yaoe_home::{HomePaths, YaoeError, YaoeResult};
use yaoe_rules::{ReqwestSrsFetcher, SrsFetcher, SrsValidator, SystemSrsValidator};
use yaoe_ssh::{SshTransport, SystemSshTransport};
use yaoe_upstream::{HttpFetcher, ReqwestFetcher};

use crate::system::{
    LocalMihomo, LocalSingBox, PublicConfigFetcher, RealityKeypairGenerator,
    ReqwestPublicConfigFetcher, SystemLocalMihomo, SystemLocalSingBox,
    SystemRealityKeypairGenerator,
};

pub struct RuntimeDeps {
    pub cloudflare: Box<dyn CloudflareZoneResolver>,
    pub r2: Box<dyn R2Wrangler>,
    pub gitee: Box<dyn yaoe_gitee::GiteeApi>,
    pub git: Box<dyn GitPublisher>,
    pub upstream_fetcher: Box<dyn HttpFetcher>,
    pub srs_fetcher: Box<dyn SrsFetcher>,
    pub srs_validator: Box<dyn SrsValidator>,
    pub ssh: Box<dyn SshTransport>,
    pub local_sing_box: Box<dyn LocalSingBox>,
    pub local_mihomo: Box<dyn LocalMihomo>,
    pub reality_keypair: Box<dyn RealityKeypairGenerator>,
    pub public_config_fetcher: Box<dyn PublicConfigFetcher>,
}

impl RuntimeDeps {
    pub fn production(config: &Config) -> YaoeResult<Self> {
        Ok(Self {
            cloudflare: Box::new(CloudflareClient::new(config.cloudflare.token.clone())?),
            r2: Box::new(SystemR2Wrangler),
            gitee: Box::new(GiteeHttpApi::new(config.gitee.token.clone())?),
            git: Box::new(SystemGitPublisher),
            upstream_fetcher: Box::new(ReqwestFetcher::new()?),
            srs_fetcher: Box::new(ReqwestSrsFetcher::new()?),
            srs_validator: Box::new(SystemSrsValidator),
            ssh: Box::new(SystemSshTransport::new()),
            local_sing_box: Box::new(SystemLocalSingBox),
            local_mihomo: Box::new(SystemLocalMihomo),
            reality_keypair: Box::new(SystemRealityKeypairGenerator),
            public_config_fetcher: Box::new(ReqwestPublicConfigFetcher::new()?),
        })
    }

    pub fn production_ssh_only() -> YaoeResult<Self> {
        Ok(Self {
            cloudflare: Box::new(NoopCloudflare),
            r2: Box::new(NoopR2),
            gitee: Box::new(NoopGitee),
            git: Box::new(NoopGit),
            upstream_fetcher: Box::new(NoopFetcher),
            srs_fetcher: Box::new(NoopSrsFetcher),
            srs_validator: Box::new(NoopSrsValidator),
            ssh: Box::new(SystemSshTransport::new()),
            local_sing_box: Box::new(SystemLocalSingBox),
            local_mihomo: Box::new(NoopMihomo),
            reality_keypair: Box::new(SystemRealityKeypairGenerator),
            public_config_fetcher: Box::new(NoopPublicConfigFetcher),
        })
    }

    pub fn production_local_validation() -> YaoeResult<Self> {
        Ok(Self {
            cloudflare: Box::new(NoopCloudflare),
            r2: Box::new(NoopR2),
            gitee: Box::new(NoopGitee),
            git: Box::new(NoopGit),
            upstream_fetcher: Box::new(NoopFetcher),
            srs_fetcher: Box::new(NoopSrsFetcher),
            srs_validator: Box::new(NoopSrsValidator),
            ssh: Box::new(SystemSshTransport::new()),
            local_sing_box: Box::new(SystemLocalSingBox),
            local_mihomo: Box::new(SystemLocalMihomo),
            reality_keypair: Box::new(SystemRealityKeypairGenerator),
            public_config_fetcher: Box::new(NoopPublicConfigFetcher),
        })
    }
}

struct NoopCloudflare;

impl CloudflareZoneResolver for NoopCloudflare {
    fn resolve_zone_id(&self, _delivery_domain: &str) -> YaoeResult<String> {
        Err(YaoeError::Internal(
            "Cloudflare resolver unavailable".into(),
        ))
    }
}

struct NoopR2;

impl R2Wrangler for NoopR2 {
    fn bucket_exists(&self, _: &str, _: &str, _: &str) -> YaoeResult<bool> {
        Err(YaoeError::Internal("R2 unavailable".into()))
    }

    fn create_bucket(&self, _: &str, _: &str, _: &str) -> YaoeResult<()> {
        Err(YaoeError::Internal("R2 unavailable".into()))
    }

    fn domain_state(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> YaoeResult<Option<yaoe_cloudflare::DomainState>> {
        Err(YaoeError::Internal("R2 unavailable".into()))
    }

    fn add_domain(&self, _: &str, _: &str, _: &str, _: &str, _: &str) -> YaoeResult<()> {
        Err(YaoeError::Internal("R2 unavailable".into()))
    }

    fn update_domain_tls(&self, _: &str, _: &str, _: &str, _: &str) -> YaoeResult<()> {
        Err(YaoeError::Internal("R2 unavailable".into()))
    }

    fn put_object(&self, _: &str, _: &str, _: &str, _: &str, _: &Path, _: &str) -> YaoeResult<()> {
        Err(YaoeError::Internal("R2 unavailable".into()))
    }
}

struct NoopGitee;

impl yaoe_gitee::GiteeApi for NoopGitee {
    fn authenticated_login(&self) -> YaoeResult<String> {
        Err(YaoeError::Internal("Gitee unavailable".into()))
    }

    fn ensure_repository(&self, _: &str, _: &str) -> YaoeResult<()> {
        Err(YaoeError::Internal("Gitee unavailable".into()))
    }

    fn ensure_release(&self, _: &str, _: &str) -> YaoeResult<yaoe_gitee::Release> {
        Err(YaoeError::Internal("Gitee unavailable".into()))
    }

    fn release_asset_names(&self, _: &str, _: &str, _: u64) -> YaoeResult<Vec<String>> {
        Err(YaoeError::Internal("Gitee unavailable".into()))
    }

    fn upload_release_asset(&self, _: &str, _: &str, _: u64, _: &Path) -> YaoeResult<()> {
        Err(YaoeError::Internal("Gitee unavailable".into()))
    }
}

struct NoopGit;

impl GitPublisher for NoopGit {
    fn ensure_branch_baseline(
        &self,
        _: &HomePaths,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &[BootstrapFile],
    ) -> YaoeResult<()> {
        Err(YaoeError::Internal("Git publisher unavailable".into()))
    }

    fn publish_bootstrap_files(
        &self,
        _: &HomePaths,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &[BootstrapFile],
    ) -> YaoeResult<()> {
        Err(YaoeError::Internal("Git publisher unavailable".into()))
    }
}

struct NoopFetcher;

impl HttpFetcher for NoopFetcher {
    fn fetch(&self, _url: &str) -> YaoeResult<Vec<u8>> {
        Err(YaoeError::Internal("HTTP fetcher unavailable".into()))
    }
}

struct NoopSrsFetcher;

impl SrsFetcher for NoopSrsFetcher {
    fn fetch_srs(&self, _url: &str) -> YaoeResult<Vec<u8>> {
        Err(YaoeError::Internal("SRS fetcher unavailable".into()))
    }
}

struct NoopSrsValidator;

impl SrsValidator for NoopSrsValidator {
    fn validate_binary_rule_set(&self, _path: &Path, _tag: &str) -> YaoeResult<()> {
        Err(YaoeError::Internal("SRS validator unavailable".into()))
    }
}

struct NoopMihomo;

impl LocalMihomo for NoopMihomo {
    fn require_version(&self) -> YaoeResult<()> {
        Err(YaoeError::Internal("mihomo unavailable".into()))
    }

    fn check_config(&self, _path: &Path) -> YaoeResult<()> {
        Err(YaoeError::Internal("mihomo unavailable".into()))
    }
}

struct NoopPublicConfigFetcher;

impl PublicConfigFetcher for NoopPublicConfigFetcher {
    fn fetch_ok(&self, _url: &str) -> YaoeResult<Option<Vec<u8>>> {
        Err(YaoeError::Internal(
            "public config fetcher unavailable".into(),
        ))
    }
}
