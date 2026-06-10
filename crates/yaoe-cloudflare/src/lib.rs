//! Cloudflare zone resolution and R2 delivery orchestration.

mod r2;

pub use r2::{
    CloudflareClient, CloudflareZoneResolver, DomainState, R2Wrangler, SystemR2Wrangler,
    public_config_object_key, public_config_url,
};
