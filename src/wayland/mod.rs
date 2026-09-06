mod handlers;
mod state;
mod surface;

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use state::State;
use wayland_client::Connection;

use crate::config::Config;

pub(crate) fn run(config: Config) -> Result<()> {
    let connection = Connection::connect_to_env().context("failed to connect to wayland")?;

    let (global_list, mut event_queue) = wayland_client::globals::registry_queue_init(&connection).context("failed to initialise globals registry")?;
    let queue_handle = event_queue.handle();
    let mut state = State::bind(&global_list, &queue_handle, config)?;
    event_queue.roundtrip(&mut state).context("roundtrip failed")?;
    state.create_surfaces(&queue_handle);

    if state.surfaces.is_empty() {
        anyhow::bail!("no surfaces were configured by the compositor");
    }

    let mut event_loop = EventLoop::try_new().context("failed to create event loop")?;
    WaylandSource::new(connection, event_queue).insert(event_loop.handle()).context("failed to insert wayland_source")?;

    event_loop.run(None, &mut state, |_| {}).context("event loop error")
}
