use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;

use super::surface::Surface;
use crate::config::Config;
use crate::image::Image;

pub(super) struct State {
    pub(super) registry_state: RegistryState,
    pub(super) output_state: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    pub(super) shm: Shm,
    surfaces: Vec<Surface>,
    config: Option<Config>,
}

impl State {
    pub(super) fn bind(global_list: &GlobalList, queue_handle: &QueueHandle<Self>) -> Result<Self> {
        let registry_state = RegistryState::new(global_list);
        let output_state = OutputState::new(global_list, queue_handle);
        let compositor = CompositorState::bind(global_list, queue_handle).context("wl_compositor not available")?;
        let layer_shell = LayerShell::bind(global_list, queue_handle).context("zwlr_layer_shell_v1 not available")?;
        let shm = Shm::bind(global_list, queue_handle).context("wl_shm not available")?;

        Ok(Self { registry_state, output_state, compositor, layer_shell, shm, surfaces: Vec::new(), config: None })
    }

    fn find_surface_mut(&mut self, wl: &WlSurface) -> Option<&mut Surface> {
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

    pub(super) fn apply_wallpaper(&mut self, config: Config, queue_handle: &QueueHandle<Self>) -> Result<()> {
        if self.surfaces.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        self.config = Some(config);
        self.render_pending(queue_handle)
    }

    fn render_pending(&mut self, queue_handle: &QueueHandle<Self>) -> Result<()> {
        let Some(config) = self.config.as_ref() else {
            return Ok(());
        };

        let pending = self.surfaces.iter().filter(|s| s.needs_render()).count();
        if pending == 0 {
            return Ok(());
        }

        tracing::info!(
            outputs = pending,
            image = %config.image.path.display(),
            strategy = ?config.resize.strategy,
            "loading and resizing wallpaper"
        );

        let image = Image::open(&config.image.path)?;
        for surface in self.surfaces.iter_mut().filter(|s| s.needs_render()) {
            surface.start_transition(&image, config, &self.shm, queue_handle)?;
        }

        Ok(())
    }

    pub(super) fn handle_configure(&mut self, wl_surface: &WlSurface, new_size: (u32, u32), queue_handle: &QueueHandle<Self>) {
        let Some(surface) = self.find_surface_mut(wl_surface) else {
            return;
        };

        surface.configure(new_size);
        if let Err(e) = self.render_pending(queue_handle) {
            tracing::warn!(?e, "failed to apply pending wallpaper");
        }
    }

    pub(super) fn handle_frame_callback(&mut self, wl_surface: &WlSurface, queue_handle: &QueueHandle<Self>) {
        let Some(surface) = self.find_surface_mut(wl_surface) else {
            return;
        };

        if let Err(e) = surface.tick(queue_handle) {
            tracing::warn!(?e, "failed to tick transition");
        }
    }

    pub(super) fn handle_scale_changed(&mut self, wl_surface: &WlSurface, new_factor: i32, queue_handle: &QueueHandle<Self>) {
        let Some(surface) = self.find_surface_mut(wl_surface) else {
            return;
        };

        surface.rescale(new_factor);
        if let Err(e) = self.render_pending(queue_handle) {
            tracing::warn!(?e, "failed to apply pending wallpaper");
        }
    }

    pub(super) fn handle_closed(&mut self, layer_surface: &LayerSurface) {
        let wl_surface = layer_surface.wl_surface();
        self.surfaces.retain(|s| s.layer_surface.wl_surface() != wl_surface);
        tracing::info!("layer surface closed by compositor, releasing resources");
    }

    pub(super) fn handle_output_destroyed(&mut self, wl_output: &WlOutput) {
        self.surfaces.retain(|s| {
            if s.output == *wl_output {
                s.destroy();
                false
            } else {
                true
            }
        });
    }
}
