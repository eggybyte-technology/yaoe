//! sing-box server and client config rendering.

mod client;
mod scripts;
mod server;

pub use client::{
    ClientPlatform, ClientRenderInput, render_android_client_config, render_clash_verge_profile,
    render_client_config, render_ios_client_config, render_linux_client_config,
    render_macos_client_config, validate_client_semantics,
};
pub use scripts::{raw_script_url, render_install_script, render_update_script};
pub use server::{
    HealthProbeRenderInput, ServerRenderInput, render_health_probe_config, render_server_config,
};
