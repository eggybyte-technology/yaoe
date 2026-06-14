//! Official CN direct sing-box SRS mirroring.

use std::path::PathBuf;
use std::process::Command;

use reqwest::blocking::Client;
use yaoe_home::{
    CN_DOMAIN_PUBLIC_ASSET, CN_DOMAIN_UPSTREAM_URL, CN_IPV4_PUBLIC_ASSET, CN_IPV4_UPSTREAM_URL,
    SING_BOX_VERSION, YaoeError, YaoeResult, atomic_write, sing_box_version_line,
};

pub trait SrsFetcher: Send + Sync {
    fn fetch_srs(&self, url: &str) -> YaoeResult<Vec<u8>>;
}

pub trait SrsValidator: Send + Sync {
    fn validate_binary_rule_set(&self, path: &std::path::Path, tag: &str) -> YaoeResult<()>;
}

pub struct ReqwestSrsFetcher {
    client: Client,
}

pub struct SystemSrsValidator;

impl ReqwestSrsFetcher {
    pub fn new() -> YaoeResult<Self> {
        let client = Client::builder()
            .user_agent(format!(
                "yaoe/{}",
                yaoe_home::YAOE_PRODUCT_REVISION.trim_start_matches('v')
            ))
            .build()
            .map_err(|e| YaoeError::SrsFetch(format!("http client: {e}")))?;
        Ok(Self { client })
    }
}

impl SrsFetcher for ReqwestSrsFetcher {
    fn fetch_srs(&self, url: &str) -> YaoeResult<Vec<u8>> {
        let resp = self
            .client
            .get(url)
            .send()
            .map_err(|e| YaoeError::SrsFetch(format!("GET {url}: {e}")))?;
        let status = resp.status();
        if status != reqwest::StatusCode::OK {
            return Err(YaoeError::SrsFetch(format!("GET {url} returned {status}")));
        }
        let bytes = resp
            .bytes()
            .map_err(|e| YaoeError::SrsFetch(format!("read {url}: {e}")))?;
        if bytes.is_empty() {
            return Err(YaoeError::SrsFetch(format!(
                "GET {url} returned empty body"
            )));
        }
        Ok(bytes.to_vec())
    }
}

impl SrsValidator for SystemSrsValidator {
    fn validate_binary_rule_set(&self, path: &std::path::Path, tag: &str) -> YaoeResult<()> {
        let version = Command::new("sing-box")
            .arg("version")
            .output()
            .map_err(|e| YaoeError::SrsFetch(format!("run sing-box version: {e}")))?;
        let first_line = String::from_utf8_lossy(&version.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        if !version.status.success() || first_line != sing_box_version_line() {
            return Err(YaoeError::SrsFetch(format!(
                "sing-box from PATH must report version {SING_BOX_VERSION} for SRS validation"
            )));
        }

        let validation_path =
            path.with_file_name(format!(".validate-{}-{}.json", tag, std::process::id()));
        let escaped_path = path.to_string_lossy();
        let config = serde_json::json!({
            "log": { "level": "error" },
            "outbounds": [
                { "type": "direct", "tag": "direct" }
            ],
            "route": {
                "rule_set": [
                    {
                        "type": "local",
                        "tag": tag,
                        "format": "binary",
                        "path": escaped_path
                    }
                ],
                "rules": [
                    { "rule_set": [tag], "action": "route", "outbound": "direct" }
                ],
                "final": "direct"
            }
        });
        let bytes = serde_json::to_vec_pretty(&config)
            .map_err(|e| YaoeError::SrsFetch(format!("render SRS validation config: {e}")))?;
        atomic_write(&validation_path, &bytes, 0o600)?;
        let output = Command::new("sing-box")
            .arg("check")
            .arg("-c")
            .arg(&validation_path)
            .output()
            .map_err(|e| YaoeError::SrsFetch(format!("run sing-box SRS validation: {e}")));
        let _ = std::fs::remove_file(&validation_path);
        let output = output?;
        if !output.status.success() {
            return Err(YaoeError::SrsFetch(format!(
                "sing-box SRS validation failed for {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SrsAsset {
    pub asset_name: &'static str,
    pub upstream_url: &'static str,
    pub cache_path: PathBuf,
    pub tag: &'static str,
}

pub fn srs_assets(paths: &yaoe_home::HomePaths) -> [SrsAsset; 2] {
    [
        SrsAsset {
            asset_name: CN_DOMAIN_PUBLIC_ASSET,
            upstream_url: CN_DOMAIN_UPSTREAM_URL,
            cache_path: paths.upstream_srs(CN_DOMAIN_PUBLIC_ASSET),
            tag: yaoe_home::CN_DOMAIN_RULE_TAG,
        },
        SrsAsset {
            asset_name: CN_IPV4_PUBLIC_ASSET,
            upstream_url: CN_IPV4_UPSTREAM_URL,
            cache_path: paths.upstream_srs(CN_IPV4_PUBLIC_ASSET),
            tag: yaoe_home::CN_IPV4_RULE_TAG,
        },
    ]
}

pub fn ensure_srs_cache<F: SrsFetcher + ?Sized, V: SrsValidator + ?Sized>(
    paths: &yaoe_home::HomePaths,
    fetcher: &F,
    validator: &V,
) -> YaoeResult<[SrsAsset; 2]> {
    let assets = srs_assets(paths);
    for asset in &assets {
        if std::fs::metadata(&asset.cache_path).is_ok_and(|m| m.is_file() && m.len() > 0)
            && validator
                .validate_binary_rule_set(&asset.cache_path, asset.tag)
                .is_ok()
        {
            continue;
        }
        let bytes = fetcher.fetch_srs(asset.upstream_url)?;
        if bytes.is_empty() {
            return Err(YaoeError::SrsFetch(format!(
                "{} returned empty body",
                asset.upstream_url
            )));
        }
        atomic_write(&asset.cache_path, &bytes, 0o644)?;
        validator.validate_binary_rule_set(&asset.cache_path, asset.tag)?;
    }
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        fetched: std::sync::Mutex<Vec<String>>,
        validated: std::sync::Mutex<Vec<String>>,
    }

    impl SrsFetcher for Fake {
        fn fetch_srs(&self, url: &str) -> YaoeResult<Vec<u8>> {
            self.fetched.lock().unwrap().push(url.to_string());
            Ok(vec![1, 2, 3])
        }
    }

    impl SrsValidator for Fake {
        fn validate_binary_rule_set(&self, path: &std::path::Path, tag: &str) -> YaoeResult<()> {
            self.validated
                .lock()
                .unwrap()
                .push(format!("{tag}:{}", path.display()));
            if std::fs::read(path).unwrap_or_default() == b"invalid" {
                Err(YaoeError::SrsFetch("invalid cached SRS".into()))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn caches_two_fixed_srs_assets() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(tmp.path().join(".yaoe"));
        let fetcher = Fake::default();
        let assets = ensure_srs_cache(&paths, &fetcher, &fetcher).unwrap();
        assert_eq!(assets[0].asset_name, "cn-domain.srs");
        assert_eq!(assets[0].upstream_url, yaoe_home::CN_DOMAIN_UPSTREAM_URL);
        assert_eq!(assets[1].asset_name, "cn-ipv4.srs");
        assert_eq!(assets[1].upstream_url, yaoe_home::CN_IPV4_UPSTREAM_URL);
        assert_eq!(std::fs::read(&assets[0].cache_path).unwrap(), vec![1, 2, 3]);
        assert_eq!(fetcher.fetched.lock().unwrap().len(), 2);
        assert_eq!(fetcher.validated.lock().unwrap().len(), 2);
    }

    #[test]
    fn valid_nonempty_cache_is_reused_after_validation() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(tmp.path().join(".yaoe"));
        for asset in srs_assets(&paths) {
            atomic_write(&asset.cache_path, b"cached", 0o644).unwrap();
        }
        let fake = Fake::default();

        ensure_srs_cache(&paths, &fake, &fake).unwrap();

        assert!(fake.fetched.lock().unwrap().is_empty());
        assert_eq!(fake.validated.lock().unwrap().len(), 2);
    }

    #[test]
    fn invalid_nonempty_cache_is_replaced_and_validated_again() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = yaoe_home::HomePaths::new(tmp.path().join(".yaoe"));
        for asset in srs_assets(&paths) {
            atomic_write(&asset.cache_path, b"invalid", 0o644).unwrap();
        }
        let fake = Fake::default();

        ensure_srs_cache(&paths, &fake, &fake).unwrap();

        assert_eq!(fake.fetched.lock().unwrap().len(), 2);
        assert_eq!(fake.validated.lock().unwrap().len(), 4);
        assert_eq!(
            std::fs::read(paths.upstream_srs(yaoe_home::CN_DOMAIN_PUBLIC_ASSET)).unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(
            std::fs::read(paths.upstream_srs(yaoe_home::CN_IPV4_PUBLIC_ASSET)).unwrap(),
            vec![1, 2, 3]
        );
    }
}
