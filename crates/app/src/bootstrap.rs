//! Startup concerns that don't belong in main or the app struct:
//! tracing/logging initialization, environment probing.

use tracing_subscriber::{fmt, EnvFilter};

pub fn install_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,roncad=debug"));
    let _ = fmt().with_env_filter(filter).try_init();
}
