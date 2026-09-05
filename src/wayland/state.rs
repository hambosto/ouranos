use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_surface::WlSurface;

use super::surface::{Status, Surface};
use crate::config::Config;
use crate::image::Image;

pub(super) struct State {
    pub(super) registry_state: RegistryState,
    pub(super) output_state: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    pub(super) shm: Shm,
    pub(super) surfaces: Vec<Surface>,
    config: Config,
}

impl State {
    pub(super) fn bind(global_list: &GlobalList, queue_handle: &QueueHandle<Self>, config: Config) -> Result<Self> {
        let registry_state = RegistryState::new(global_list);
        let output_state = OutputState::new(global_list, queue_handle);
        let compositor = CompositorState::bind(global_list, queue_handle).context("wl_compositor not available")?;
        let layer_shell = LayerShell::bind(global_list, queue_handle).context("zwlr_layer_shell_v1 not available")?;
        let shm = Shm::bind(global_list, queue_handle).context("wl_shm not available")?;

        Ok(Self { registry_state, output_state, compositor, layer_shell, shm, surfaces: Vec::new(), config })
    }

    pub(super) fn find_surface_mut(&mut self, wl: &WlSurface) -> Option<&mut Surface> {
        self.surfaces.iter_mut().find(|s| s.layer_surface.wl_surface() == wl)
    }

    pub(super) fn create_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        for output in self.output_state.outputs() {
            if self.surfaces.iter().any(|s| s.output == output) {
                continue;
            }

            let Some(info) = self.output_state.info(&output) else {
                continue;
            };

            if let Some(surface) = Surface::create(&self.compositor, &self.layer_shell, output, &info, queue_handle) {
                self.surfaces.push(surface);
            }
        }
    }

    pub(super) fn render_pending(&mut self, queue_handle: &QueueHandle<Self>) -> Result<()> {
        let pending = self.surfaces.iter().filter(|s| matches!(s.status, Status::Pending)).count();
        if pending == 0 {
            return Ok(());
        }

        tracing::info!(
            outputs = pending,
            image = %self.config.image.display(),
            strategy = ?self.config.resize.strategy,
            "loading and resizing wallpaper"
        );

        let image = Image::open(&self.config.image)?;
        for surface in self.surfaces.iter_mut().filter(|s| matches!(s.status, Status::Pending)) {
            surface.start_transition(&image, &self.config, &self.shm, queue_handle)?;
        }

        Ok(())
    }

    pub(super) fn render(&mut self, queue_handle: &QueueHandle<Self>) {
        if let Err(e) = self.render_pending(queue_handle) {
            tracing::warn!(?e, "failed to apply pending wallpaper");
        }
    }
}
