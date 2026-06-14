use std::path::Path;
use std::thread;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, Response, multipart};
use serde::Deserialize;
use yaoe_home::{
    GITEE_BOOTSTRAP_BRANCH, GITEE_RELEASE_TAG, LogLevel, YAOE_PRODUCT_REVISION, YaoeError,
    YaoeResult, log_event as home_log_event,
};

const GITEE_HTTP_ATTEMPTS: usize = 8;

#[derive(Debug, Clone)]
pub struct Release {
    pub id: u64,
}

pub trait GiteeApi: Send + Sync {
    fn authenticated_login(&self) -> YaoeResult<String>;
    fn ensure_repository(&self, owner: &str, repo: &str) -> YaoeResult<()>;
    fn ensure_release(&self, owner: &str, repo: &str) -> YaoeResult<Release>;
    fn release_asset_names(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
    ) -> YaoeResult<Vec<String>>;
    fn upload_release_asset(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        file: &Path,
    ) -> YaoeResult<()>;
}

pub struct GiteeHttpApi {
    client: Client,
    token: String,
}

impl GiteeHttpApi {
    pub fn new(token: impl Into<String>) -> YaoeResult<Self> {
        let client = Client::builder()
            .user_agent(format!(
                "yaoe/{}",
                YAOE_PRODUCT_REVISION.trim_start_matches('v')
            ))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|e| YaoeError::Gitee(format!("http client: {e}")))?;
        Ok(Self {
            client,
            token: token.into(),
        })
    }

    fn url(&self, path: &str) -> String {
        let sep = if path.contains('?') { '&' } else { '?' };
        format!(
            "https://gitee.com/api/v5{path}{sep}access_token={}",
            self.token
        )
    }

    fn redact(&self, text: impl AsRef<str>) -> String {
        text.as_ref().replace(&self.token, "<redacted>")
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> YaoeResult<Option<T>> {
        let resp = self.send_with_retry("GET", path, || self.client.get(self.url(path)))?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let status = resp.status();
        let text = resp
            .text()
            .map_err(|e| YaoeError::Gitee(format!("read response {path}: {e}")))?;
        if !status.is_success() {
            let text = self.redact(text);
            return Err(YaoeError::Gitee(format!(
                "Gitee {path} returned {status}: {text}"
            )));
        }
        if text.trim() == "null" {
            return Ok(None);
        }
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| YaoeError::Gitee(format!("parse {path}: {e}")))
    }

    fn post_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> YaoeResult<T> {
        let resp =
            self.send_with_retry("POST", path, || self.client.post(self.url(path)).json(body))?;
        parse_json(resp, path, &self.token)
    }

    fn send_with_retry(
        &self,
        method: &str,
        path: &str,
        build: impl Fn() -> reqwest::blocking::RequestBuilder,
    ) -> YaoeResult<Response> {
        let mut last_error = None;
        for attempt in 1..=GITEE_HTTP_ATTEMPTS {
            match build().send() {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let err_text = self.redact(err.to_string());
                    log_event(
                        "gitee",
                        "http",
                        &[
                            ("method", method.to_string()),
                            ("path", path.to_string()),
                            ("attempt", format!("{attempt}/{GITEE_HTTP_ATTEMPTS}")),
                            ("error", err_text.clone()),
                        ],
                    );
                    last_error = Some(err_text);
                    if attempt != GITEE_HTTP_ATTEMPTS {
                        thread::sleep(Duration::from_secs((attempt.min(5) * 3) as u64));
                    }
                }
            }
        }
        Err(YaoeError::Gitee(format!(
            "{method} {path}: {}",
            last_error.as_deref().unwrap_or("<none>")
        )))
    }
}

impl GiteeApi for GiteeHttpApi {
    fn authenticated_login(&self) -> YaoeResult<String> {
        let user: User = self
            .get_json("/user")?
            .ok_or_else(|| YaoeError::Gitee("authenticated Gitee user not found".into()))?;
        Ok(user.login)
    }

    fn ensure_repository(&self, owner: &str, repo: &str) -> YaoeResult<()> {
        let login = self.authenticated_login()?;
        if self
            .get_json::<Repository>(&format!("/repos/{owner}/{repo}"))?
            .is_some()
        {
            return Ok(());
        }
        let body = public_repository_body(repo);
        if owner == login {
            let _: Repository = self.post_json("/user/repos", &body)?;
        } else {
            let _: Repository = self.post_json(&format!("/orgs/{owner}/repos"), &body)?;
        }
        Ok(())
    }

    fn ensure_release(&self, owner: &str, repo: &str) -> YaoeResult<Release> {
        if let Some(release) = self.get_json::<ReleaseResource>(&format!(
            "/repos/{owner}/{repo}/releases/tags/{GITEE_RELEASE_TAG}"
        ))? {
            return Ok(Release { id: release.id });
        }
        let body = serde_json::json!({
            "tag_name": GITEE_RELEASE_TAG,
            "name": GITEE_RELEASE_TAG,
            "body": "YAOE v0.0.1 delivery assets for sing-box 1.13.13",
            "target_commitish": GITEE_BOOTSTRAP_BRANCH,
            "prerelease": false
        });
        let release: ReleaseResource =
            self.post_json(&format!("/repos/{owner}/{repo}/releases"), &body)?;
        Ok(Release { id: release.id })
    }

    fn release_asset_names(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
    ) -> YaoeResult<Vec<String>> {
        let assets: Vec<Attachment> = self
            .get_json(&format!(
                "/repos/{owner}/{repo}/releases/{release_id}/attach_files"
            ))?
            .unwrap_or_default();
        Ok(assets.into_iter().map(|a| a.name).collect())
    }

    fn upload_release_asset(
        &self,
        owner: &str,
        repo: &str,
        release_id: u64,
        file: &Path,
    ) -> YaoeResult<()> {
        let file_name = file
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| YaoeError::Gitee(format!("invalid asset path {}", file.display())))?;
        let path = format!("/repos/{owner}/{repo}/releases/{release_id}/attach_files");
        let mut last_send_error = None;
        let mut response = None;
        for attempt in 1..=GITEE_HTTP_ATTEMPTS {
            let part = multipart::Part::file(file)
                .map_err(|e| YaoeError::Gitee(format!("open asset {}: {e}", file.display())))?
                .file_name(file_name.to_string());
            let form = multipart::Form::new().part("file", part);
            match self.client.post(self.url(&path)).multipart(form).send() {
                Ok(resp) => {
                    response = Some(resp);
                    break;
                }
                Err(err) => {
                    let err_text = self.redact(err.to_string());
                    log_event(
                        "gitee",
                        "release",
                        &[
                            ("asset", file_name.to_string()),
                            ("attempt", format!("{attempt}/{GITEE_HTTP_ATTEMPTS}")),
                            ("error", err_text.clone()),
                        ],
                    );
                    last_send_error = Some(err_text);
                    if attempt != GITEE_HTTP_ATTEMPTS {
                        thread::sleep(Duration::from_secs((attempt.min(5) * 3) as u64));
                    }
                }
            }
        }
        let resp = response.ok_or_else(|| {
            YaoeError::Gitee(format!(
                "upload asset {file_name}: {}",
                last_send_error.as_deref().unwrap_or("<none>")
            ))
        })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = self.redact(resp.text().unwrap_or_default());
            return Err(YaoeError::Gitee(format!(
                "upload asset {file_name} failed {status}: {text}"
            )));
        }
        Ok(())
    }
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    resp: reqwest::blocking::Response,
    path: &str,
    token: &str,
) -> YaoeResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .map_err(|e| YaoeError::Gitee(format!("read response {path}: {e}")))?;
    if !status.is_success() {
        let text = text.replace(token, "<redacted>");
        return Err(YaoeError::Gitee(format!(
            "Gitee {path} returned {status}: {text}"
        )));
    }
    serde_json::from_str(&text).map_err(|e| YaoeError::Gitee(format!("parse {path}: {e}")))
}

fn public_repository_body(repo: &str) -> serde_json::Value {
    serde_json::json!({
        "name": repo,
        "private": false,
        "has_issues": false,
        "has_wiki": false,
        "auto_init": false
    })
}

fn log_event(command: &str, stage: &str, pairs: &[(&str, String)]) {
    home_log_event(
        LogLevel::Warn,
        &format!("{command}.{stage}"),
        "retrying request",
        pairs,
    );
}

#[derive(Debug, Deserialize)]
struct User {
    login: String,
}

#[derive(Debug, Deserialize)]
struct Repository {}

#[derive(Debug, Deserialize)]
struct ReleaseResource {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct Attachment {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_create_body_is_public_without_auto_init() {
        let body = public_repository_body("yaoe-delivery");
        assert_eq!(body["name"], "yaoe-delivery");
        assert_eq!(body["private"], false);
        assert_eq!(body["auto_init"], false);
    }
}
