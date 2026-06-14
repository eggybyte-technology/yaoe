use yaoe_home::sha256_hex;

use crate::builder::PackageBuildInput;

pub fn package_input_digest(input: &PackageBuildInput) -> String {
    let material = format!(
        "{}|{}|{}|{}",
        input.server_name, input.runtime_variant, input.config_sha256, input.sing_box_sha256,
    );
    sha256_hex(material.as_bytes())
}
