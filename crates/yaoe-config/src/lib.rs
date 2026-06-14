//! Parse and validate `.yaoe/yaoe.toml`.

mod model;
mod validate;

pub use model::{
    CloudflareConfig, Config, CredentialConfig, GiteeConfig, RealityConfig, RouteConfig,
    ServerConfig, SshConfig,
};
pub use validate::{
    ClientEntrypointParts, atomic_update_reality_keypair, atomic_update_toml_field,
    derive_reality_public_key, generate_config_key, generate_reality_short_id,
    generate_server_port, generate_vless_uuid, load_and_validate, parse_and_validate,
    parse_for_client_entrypoints, validate_config,
};
