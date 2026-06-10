use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;
use yaoe_home::{
    CN_DOMAIN_PUBLIC_ASSET, CN_DOMAIN_UPSTREAM_URL, CN_IPV4_PUBLIC_ASSET, CN_IPV4_UPSTREAM_URL,
    YaoeError, YaoeResult, atomic_write, release_asset_name, service_variants,
    upstream_sing_box_url,
};

pub trait HttpFetcher: Send + Sync {
    fn fetch(&self, url: &str) -> YaoeResult<Vec<u8>>;
}

pub struct ReqwestFetcher {
    client: Client,
}

impl ReqwestFetcher {
    pub fn new() -> YaoeResult<Self> {
        let client = Client::builder()
            .user_agent(format!(
                "yaoe/{}",
                yaoe_home::YAOE_PRODUCT_REVISION.trim_start_matches('v')
            ))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| YaoeError::Upstream(format!("http client: {e}")))?;
        Ok(Self { client })
    }
}

impl HttpFetcher for ReqwestFetcher {
    fn fetch(&self, url: &str) -> YaoeResult<Vec<u8>> {
        let mut last_error = None;
        for attempt in 1..=3 {
            match self.client.get(url).send() {
                Ok(resp) => {
                    let status = resp.status();
                    if status != reqwest::StatusCode::OK {
                        last_error = Some(format!("GET {url} returned {status}"));
                    } else {
                        let bytes = resp
                            .bytes()
                            .map_err(|e| YaoeError::Upstream(format!("read {url}: {e}")))?;
                        if bytes.is_empty() {
                            return Err(YaoeError::Upstream(format!(
                                "GET {url} returned empty body"
                            )));
                        }
                        return Ok(bytes.to_vec());
                    }
                }
                Err(err) => {
                    last_error = Some(format!("GET {url}: {err}"));
                }
            }
            if attempt != 3 {
                std::thread::sleep(Duration::from_secs(attempt * 2));
            }
        }
        Err(YaoeError::Upstream(format!(
            "failed to fetch upstream artifact after 3 attempts: {}",
            last_error.unwrap_or_else(|| "unknown error".to_string())
        )))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAssetKind {
    SingBox,
    Srs,
}

#[derive(Debug, Clone)]
pub struct RuntimeArtifact {
    pub asset_name: String,
    pub upstream_url: String,
    pub cache_path: PathBuf,
    pub kind: RuntimeAssetKind,
}

pub fn runtime_assets(paths: &yaoe_home::HomePaths) -> Vec<RuntimeArtifact> {
    let mut out = Vec::new();
    for variant in service_variants() {
        let asset_name = release_asset_name(variant.id)
            .expect("constant platform has asset")
            .to_string();
        out.push(RuntimeArtifact {
            upstream_url: upstream_sing_box_url(variant.id).expect("constant variant has URL"),
            cache_path: paths.upstream_sing_box_archive(variant.id),
            asset_name,
            kind: RuntimeAssetKind::SingBox,
        });
    }
    out.push(RuntimeArtifact {
        asset_name: CN_DOMAIN_PUBLIC_ASSET.to_string(),
        upstream_url: CN_DOMAIN_UPSTREAM_URL.to_string(),
        cache_path: paths.upstream_srs(CN_DOMAIN_PUBLIC_ASSET),
        kind: RuntimeAssetKind::Srs,
    });
    out.push(RuntimeArtifact {
        asset_name: CN_IPV4_PUBLIC_ASSET.to_string(),
        upstream_url: CN_IPV4_UPSTREAM_URL.to_string(),
        cache_path: paths.upstream_srs(CN_IPV4_PUBLIC_ASSET),
        kind: RuntimeAssetKind::Srs,
    });
    out
}

pub fn ensure_runtime_artifacts(
    paths: &yaoe_home::HomePaths,
    fetcher: &dyn HttpFetcher,
) -> YaoeResult<Vec<RuntimeArtifact>> {
    let assets = runtime_assets(paths);
    for asset in &assets {
        if asset.kind == RuntimeAssetKind::Srs {
            continue;
        }
        if non_empty_file(&asset.cache_path) {
            continue;
        }
        let bytes = fetcher.fetch(&asset.upstream_url).map_err(|e| match e {
            YaoeError::Upstream(message) if asset.kind != RuntimeAssetKind::Srs => {
                YaoeError::Upstream(message)
            }
            YaoeError::SrsFetch(message) if asset.kind == RuntimeAssetKind::Srs => {
                YaoeError::SrsFetch(message)
            }
            other if asset.kind == RuntimeAssetKind::Srs => YaoeError::SrsFetch(other.to_string()),
            other => YaoeError::Upstream(other.to_string()),
        })?;
        fetch_to_cache(&asset.cache_path, &bytes, asset.kind)?;
    }
    Ok(assets)
}

pub fn fetch_to_cache(path: &Path, bytes: &[u8], kind: RuntimeAssetKind) -> YaoeResult<()> {
    if bytes.is_empty() {
        return Err(match kind {
            RuntimeAssetKind::Srs => YaoeError::SrsFetch("empty upstream response".into()),
            _ => YaoeError::Upstream("empty upstream response".into()),
        });
    }
    atomic_write(path, bytes, 0o644)
}

pub fn non_empty_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeFetcher {
        urls: Mutex<Vec<String>>,
    }

    impl HttpFetcher for FakeFetcher {
        fn fetch(&self, url: &str) -> YaoeResult<Vec<u8>> {
            self.urls.lock().unwrap().push(url.to_string());
            Ok(format!("bytes for {url}").into_bytes())
        }
    }

    #[test]
    fn runtime_asset_plan_contains_exact_public_assets_in_order() {
        let paths = yaoe_home::HomePaths::new(".yaoe");
        let assets = runtime_assets(&paths);
        let names: Vec<_> = assets
            .iter()
            .map(|asset| asset.asset_name.as_str())
            .collect();

        assert_eq!(names, yaoe_home::all_release_asset_names());
        assert_eq!(
            assets[2].upstream_url,
            "https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.13-darwin-amd64.tar.gz"
        );
        assert_eq!(assets[2].asset_name, "sing-box-1.13.13-macos-amd64.tar.gz");
        assert_eq!(assets[4].asset_name, "cn-domain.srs");
        assert_eq!(assets[4].upstream_url, yaoe_home::CN_DOMAIN_UPSTREAM_URL);
        assert_eq!(assets[5].asset_name, "cn-ipv4.srs");
        assert_eq!(assets[5].upstream_url, yaoe_home::CN_IPV4_UPSTREAM_URL);
        let rendered_urls = assets
            .iter()
            .map(|asset| asset.upstream_url.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in [
            "apple.china.conf.srs",
            "google.china.conf.srs",
            "gfwlist.txt.srs",
            "filter.txt.srs",
            "pro.txt.srs",
            "apnic-cn-ipv4.srs",
            "apnic-cn-ipv6.srs",
            "maxmind-cn-ipv4.srs",
            "maxmind-cn-ipv6.srs",
            "ipinfo-lite-cn-ipv4.srs",
            "ipinfo-lite-cn-ipv6.srs",
        ] {
            assert!(
                !rendered_urls.contains(forbidden),
                "runtime asset plan contains forbidden source {forbidden}"
            );
        }
    }

    #[test]
    fn runtime_artifact_fetch_reuses_nonempty_cache_and_skips_srs() {
        let dir = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(dir.path().join(".yaoe"));
        for asset in runtime_assets(&paths)
            .into_iter()
            .filter(|asset| asset.kind != RuntimeAssetKind::Srs)
        {
            atomic_write(&asset.cache_path, b"cached", 0o644).unwrap();
        }
        let fetcher = FakeFetcher::default();

        let assets = ensure_runtime_artifacts(&paths, &fetcher).unwrap();

        assert_eq!(assets.len(), 6);
        assert!(fetcher.urls.lock().unwrap().is_empty());
    }

    #[test]
    fn runtime_artifact_fetch_writes_missing_non_srs_assets() {
        let dir = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(dir.path().join(".yaoe"));
        let fetcher = FakeFetcher::default();

        let assets = ensure_runtime_artifacts(&paths, &fetcher).unwrap();
        let fetched = fetcher.urls.lock().unwrap().clone();

        assert_eq!(fetched.len(), 4);
        assert!(fetched.iter().all(|url| !url.contains("rule-set")));
        for asset in assets
            .into_iter()
            .filter(|asset| asset.kind != RuntimeAssetKind::Srs)
        {
            assert!(non_empty_file(&asset.cache_path));
        }
    }
}
