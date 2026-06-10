use yaoe_home::{GITEE_BOOTSTRAP_BRANCH, GITEE_RELEASE_TAG};

pub fn raw_url(owner: &str, repo: &str, path: &str) -> String {
    format!("https://gitee.com/{owner}/{repo}/raw/{GITEE_BOOTSTRAP_BRANCH}/{path}")
}

pub fn release_asset_url(owner: &str, repo: &str, asset_name: &str) -> String {
    format!("https://gitee.com/{owner}/{repo}/releases/download/{GITEE_RELEASE_TAG}/{asset_name}")
}
