use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
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
    pub(super) compositor_state: CompositorState,
    pub(super) layer_shell: LayerShell,
    pub(super) shm_state: Shm,
    output_surfaces: Vec<Surface>,
    config: Config,
}

impl State {
    pub(super) fn new(globals: &GlobalList, qh: &QueueHandle<Self>, config: Config) -> Result<Self> {
        let registry_state = RegistryState::new(globals);
        let output_state = OutputState::new(globals, qh);
        let compositor_state = CompositorState::bind(globals, qh).context("wl_compositor not available")?;
        let layer_shell = LayerShell::bind(globals, qh).context("zwlr_layer_shell_v1 not available")?;
        let shm_state = Shm::bind(globals, qh).context("wl_shm not available")?;

        Ok(Self { registry_state, output_state, compositor_state, layer_shell, shm_state, output_surfaces: Vec::new(), config })
    }

    pub(super) fn has_output_surfaces(&self) -> bool {
        !self.output_surfaces.is_empty()
    }

    pub(super) fn ensure_output_surfaces(&mut self, qh: &QueueHandle<Self>) {
        for output in self.output_state.outputs() {
            if self.output_surfaces.iter().any(|surface| surface.is_wl_output(&output)) {
                continue;
            }

            let Some(info) = self.output_state.info(&output) else {
                continue;
            };

            if let Some(surface) = Surface::new(&self.compositor_state, &self.layer_shell, output, &info, qh) {
                self.output_surfaces.push(surface);
            }
        }
    }

    pub(super) fn remove_output_surface(&mut self, output: &WlOutput) {
        let before = self.output_surfaces.len();
        self.output_surfaces.retain(|surface| !surface.is_wl_output(output));

        if self.output_surfaces.len() != before {
            tracing::info!("wallpaper surface destroyed for disconnected output");
        }
    }

    pub(super) fn rescale_surface(&mut self, wl_surface: &WlSurface, new_factor: i32, qh: &QueueHandle<Self>) {
        let Some(index) = self.output_surfaces.iter().position(|surface| surface.is_wl_surface(wl_surface)) else {
            return;
        };

        self.output_surfaces[index].rescale(new_factor);
        self.render_pending_surfaces(qh);
    }

    pub(super) fn refresh_surface_size(&mut self, wl_surface: &WlSurface, qh: &QueueHandle<Self>) {
        let new_size = {
            let Some(index) = self.output_surfaces.iter().position(|surface| surface.is_wl_surface(wl_surface)) else {
                return;
            };

            let output = self.output_surfaces[index].wl_output().clone();
            self.output_state.info(&output).and_then(|info| {
                info.logical_size
                    .filter(|(width, height)| *width > 0 && *height > 0)
                    .map(|(width, height)| (width.cast_unsigned(), height.cast_unsigned()))
            })
        };

        let Some(new_size) = new_size else {
            return;
        };

        let Some(index) = self.output_surfaces.iter().position(|surface| surface.is_wl_surface(wl_surface)) else {
            return;
        };

        self.output_surfaces[index].configure(new_size);
        self.render_pending_surfaces(qh);
    }

    pub(super) fn advance_surface_frame(&mut self, wl_surface: &WlSurface, qh: &QueueHandle<Self>) {
        let Some(index) = self.output_surfaces.iter().position(|surface| surface.is_wl_surface(wl_surface)) else {
            return;
        };

        self.output_surfaces[index].advance_frame(qh);
    }

    pub(super) fn configure_surface(&mut self, layer: &LayerSurface, new_size: (u32, u32), qh: &QueueHandle<Self>) {
        let Some(index) = self.output_surfaces.iter().position(|surface| surface.is_layer_surface(layer)) else {
            return;
        };

        self.output_surfaces[index].configure(new_size);
        self.render_pending_surfaces(qh);
    }

    pub(super) fn drop_closed_surface(&mut self, layer: &LayerSurface, qh: &QueueHandle<Self>) {
        let before = self.output_surfaces.len();
        self.output_surfaces.retain(|surface| !surface.is_layer_surface(layer));

        if self.output_surfaces.len() != before {
            tracing::info!("layer surface closed by compositor, releasing resources");
            self.ensure_output_surfaces(qh);
        }
    }

    pub(super) fn render_pending_surfaces(&mut self, qh: &QueueHandle<Self>) {
        if !self.output_surfaces.iter().any(Surface::is_pending) {
            return;
        }

        tracing::info!(
            image = %self.config.image.display(),
            strategy = ?self.config.resize.strategy,
            "loading and resizing wallpaper"
        );

        let image = match Image::open(&self.config.image) {
            Ok(image) => image,
            Err(e) => {
                tracing::warn!(?e, "failed to apply pending wallpaper");
                return;
            }
        };

        for index in 0..self.output_surfaces.len() {
            if !self.output_surfaces[index].is_pending() {
                continue;
            }

            self.output_surfaces[index].start_transition(&image, &self.config, &self.shm_state, qh);
        }
    }
}
