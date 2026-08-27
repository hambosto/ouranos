mod config;
mod image;
mod transition;
mod wayland;

use anyhow::{Context, Result};
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

use crate::config::Config;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_file(true).with_line_number(true).init();

    let version = option_env!("OURANOS_BUILD_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
    tracing::info!(version, "ouranos starting");

    let config = Config::load().context("failed to load config")?;
    tracing::info!(
        image = %config.image.path.display(),
        transition = ?config.transition.transition_type,
        resize = ?config.resize.strategy,
        duration = config.transition.duration,
        "configuration loaded"
    );

    wayland::run(config).context("failed to run wayland wallpaper setter")
}
