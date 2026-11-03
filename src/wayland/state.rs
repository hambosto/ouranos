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

use super::surface::{ManagedSurface, Surface};
use crate::config::Config;
use crate::image::Image;

pub(super) struct State {
    pub(super) registry_state: RegistryState,
    pub(super) output_state: OutputState,
    pub(super) compositor: CompositorState,
    pub(super) layer_shell: LayerShell,
    pub(super) shm: Shm,
    surfaces: Vec<ManagedSurface>,
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

    pub(super) fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    pub(super) fn ensure_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        for output in self.output_state.outputs() {
            if self.surfaces.iter().any(|surface| surface.is_output(&output)) {
                continue;
            }
            let Some(info) = self.output_state.info(&output) else {
                continue;
            };
            if let Some(surface) = Surface::new(&self.compositor, &self.layer_shell, output, &info, queue_handle) {
                self.surfaces.push(ManagedSurface::Unconfigured(surface));
            }
        }
    }

    pub(super) fn remove_output(&mut self, output: &WlOutput) {
        let before = self.surfaces.len();
        self.surfaces.retain(|surface| !surface.is_output(output));
        if self.surfaces.len() != before {
            tracing::info!("wallpaper surface destroyed for disconnected output");
        }
    }

    pub(super) fn apply_scale(&mut self, surface: &WlSurface, scale_factor: i32, queue_handle: &QueueHandle<Self>) {
        let Some(index) = self.surfaces.iter().position(|candidate| candidate.is_wl_surface(surface)) else {
            return;
        };
        let managed = self.surfaces.remove(index).rescale(scale_factor);
        self.surfaces.insert(index, managed);
        self.render_pending(queue_handle);
    }

    pub(super) fn apply_transform(&mut self, surface: &WlSurface, queue_handle: &QueueHandle<Self>) {
        let logical_size = {
            let Some(index) = self.surfaces.iter().position(|candidate| candidate.is_wl_surface(surface)) else {
                return;
            };
            let output = self.surfaces[index].output().clone();
            self.output_state.info(&output).and_then(|info| {
                info.logical_size
                    .filter(|(width, height)| *width > 0 && *height > 0)
                    .map(|(width, height)| (width.cast_unsigned(), height.cast_unsigned()))
            })
        };
        let Some(logical_size) = logical_size else {
            return;
        };
        let Some(index) = self.surfaces.iter().position(|candidate| candidate.is_wl_surface(surface)) else {
            return;
        };
        let managed = self.surfaces.remove(index).configure(logical_size);
        self.surfaces.insert(index, managed);
        self.render_pending(queue_handle);
    }

    pub(super) fn advance_frame(&mut self, surface: &WlSurface, queue_handle: &QueueHandle<Self>) {
        let Some(index) = self.surfaces.iter().position(|candidate| candidate.is_wl_surface(surface)) else {
            return;
        };
        let managed = self.surfaces.remove(index).advance_frame(queue_handle);
        self.surfaces.insert(index, managed);
    }

    pub(super) fn apply_configure(&mut self, layer: &LayerSurface, logical_size: (u32, u32), queue_handle: &QueueHandle<Self>) {
        let Some(index) = self.surfaces.iter().position(|candidate| candidate.is_layer_surface(layer)) else {
            return;
        };
        let managed = self.surfaces.remove(index).configure(logical_size);
        self.surfaces.insert(index, managed);
        self.render_pending(queue_handle);
    }

    pub(super) fn handle_close(&mut self, layer: &LayerSurface, queue_handle: &QueueHandle<Self>) {
        let before = self.surfaces.len();
        self.surfaces.retain(|surface| !surface.is_layer_surface(layer));
        if self.surfaces.len() != before {
            tracing::info!("layer surface closed by compositor, releasing resources");
            self.ensure_surfaces(queue_handle);
        }
    }

    pub(super) fn render_pending(&mut self, queue_handle: &QueueHandle<Self>) {
        if !self.surfaces.iter().any(ManagedSurface::is_pending) {
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

        for index in 0..self.surfaces.len() {
            if !self.surfaces[index].is_pending() {
                continue;
            }
            let managed = self.surfaces.remove(index).start_transition(&image, &self.config, &self.shm, queue_handle);
            self.surfaces.insert(index, managed);
        }
    }
}
