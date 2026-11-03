use smithay_client_toolkit::compositor::{CompositorState, Region};
use smithay_client_toolkit::output::OutputInfo;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;

use super::frame::Frame;
use super::state::State;
use crate::config::Config;
use crate::image::Image;

pub(super) enum SurfaceStatus {
    Unconfigured,
    Pending,
    Animating { frame: Frame },
    Ready,
}

pub(super) struct Surface {
    layer_surface: LayerSurface,
    wl_output: WlOutput,
    logical_size: (u32, u32),
    buffer_scale: u32,
    status: SurfaceStatus,
}

impl Surface {
    pub(super) fn new(compositor_state: &CompositorState, layer_shell: &LayerShell, output: WlOutput, info: &OutputInfo, qh: &QueueHandle<State>) -> Option<Self> {
        let name = info.name.as_deref().unwrap_or("unknown");
        let description = info.description.as_deref().unwrap_or("unknown");

        let Some(logical_size) = info
            .logical_size
            .filter(|(width, height)| *width > 0 && *height > 0)
            .map(|(width, height)| (width.cast_unsigned(), height.cast_unsigned()))
            .or_else(|| {
                let fallback_scale = info.scale_factor.max(1);
                info.modes
                    .iter()
                    .find(|mode| mode.current)
                    .map(|mode| ((mode.dimensions.0 / fallback_scale).max(1).cast_unsigned(), (mode.dimensions.1 / fallback_scale).max(1).cast_unsigned()))
            })
        else {
            tracing::warn!(name, "no valid dimensions, skipping output");
            return None;
        };

        let requested_scale: u32 = info.scale_factor.max(1).cast_unsigned();
        let surface = compositor_state.create_surface(qh);
        let layer_surface = layer_shell.create_layer_surface(qh, surface, Layer::Background, Some(env!("CARGO_PKG_NAME")), Some(&output));

        layer_surface.set_anchor(Anchor::all());
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_size(0, 0);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        if let Ok(region) = Region::new(compositor_state) {
            layer_surface.set_input_region(Some(region.wl_region()));
        } else {
            tracing::warn!(name, "failed to create input region, surface may take input");
        }

        let applied_scale = layer_surface.set_buffer_scale(requested_scale).map_or(1, |()| requested_scale);
        layer_surface.commit();

        let (logical_width, logical_height) = logical_size;
        tracing::info!(name, description, logical_width, logical_height, applied_scale, "monitor detected, creating output surface");

        Some(Self { layer_surface, wl_output: output, logical_size, buffer_scale: applied_scale, status: SurfaceStatus::Unconfigured })
    }

    pub(super) fn is_wl_surface(&self, surface: &WlSurface) -> bool {
        self.layer_surface.wl_surface() == surface
    }

    pub(super) fn is_layer_surface(&self, layer: &LayerSurface) -> bool {
        self.layer_surface.wl_surface() == layer.wl_surface()
    }

    pub(super) fn is_wl_output(&self, output: &WlOutput) -> bool {
        &self.wl_output == output
    }

    pub(super) fn wl_output(&self) -> &WlOutput {
        &self.wl_output
    }

    pub(super) fn is_pending(&self) -> bool {
        matches!(self.status, SurfaceStatus::Pending)
    }

    pub(super) fn configure(&mut self, new_size: (u32, u32)) {
        let (logical_width, logical_height) = new_size;
        let (current_width, current_height) = self.logical_size;

        if logical_width > 0 && logical_height > 0 && (logical_width != current_width || logical_height != current_height) {
            tracing::info!(old_width = current_width, old_height = current_height, logical_width, logical_height, "compositor requested new surface size, resizing");
            self.logical_size = new_size;
            self.status = SurfaceStatus::Pending;
        } else if matches!(self.status, SurfaceStatus::Unconfigured) {
            self.status = SurfaceStatus::Pending;
        }
    }

    pub(super) fn rescale(&mut self, new_factor: i32) {
        let buffer_scale: u32 = new_factor.max(1).cast_unsigned();
        if buffer_scale == self.buffer_scale {
            return;
        }

        if let Err(e) = self.layer_surface.set_buffer_scale(buffer_scale) {
            tracing::warn!(?e, "compositor does not support buffer scaling, keeping current resolution");
            return;
        }

        tracing::info!(old_scale = self.buffer_scale, buffer_scale, "output scale changed, resizing");
        self.buffer_scale = buffer_scale;
        self.status = SurfaceStatus::Pending;
    }

    pub(super) fn start_transition(&mut self, image: &Image, config: &Config, shm: &Shm, qh: &QueueHandle<State>) {
        if !matches!(self.status, SurfaceStatus::Pending) {
            return;
        }

        let (logical_width, logical_height) = self.logical_size;
        let Some(buffer_width) = logical_width.checked_mul(self.buffer_scale) else {
            tracing::warn!("surface dimensions too large");
            return;
        };

        let Some(buffer_height) = logical_height.checked_mul(self.buffer_scale) else {
            tracing::warn!("surface dimensions too large");
            return;
        };

        let frame = match Frame::new(image, config, shm, buffer_width, buffer_height) {
            Ok(frame) => frame,
            Err(e) => {
                tracing::warn!(?e, "failed to start transition for output");
                return;
            }
        };

        self.status = SurfaceStatus::Animating { frame };
        self.advance_frame(qh);
    }

    pub(super) fn advance_frame(&mut self, qh: &QueueHandle<State>) {
        let mut frame = match std::mem::replace(&mut self.status, SurfaceStatus::Pending) {
            SurfaceStatus::Animating { frame } => frame,
            previous_status => {
                self.status = previous_status;
                return;
            }
        };

        match frame.advance(&self.layer_surface, qh) {
            Ok(true) => {
                self.status = SurfaceStatus::Ready;
            }

            Ok(false) => {
                self.status = SurfaceStatus::Animating { frame };
            }

            Err(e) => {
                tracing::warn!(?e, "failed to tick transition, will retry on next render");
                self.status = SurfaceStatus::Pending;
            }
        }
    }
}
