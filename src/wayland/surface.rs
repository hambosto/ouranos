use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::{CompositorState, FrameCallbackData};
use smithay_client_toolkit::output::OutputInfo;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface, SurfaceKind};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::Format;

use super::state::State;
use crate::config::Config;
use crate::image::Image;
use crate::transition::Transition;

enum Status {
    Unconfigured,
    NeedsRender,
    Transitioning { transition: Box<Transition>, pool: SlotPool, pixels: Vec<u8> },
    Complete,
}

pub(super) struct Surface {
    pub(super) layer_surface: LayerSurface,
    pub(super) output: WlOutput,
    output_name: String,
    width: u32,
    height: u32,
    scale: u32,
    status: Status,
}

impl Surface {
    pub(super) fn create(compositor: &CompositorState, layer_shell: &LayerShell, output: WlOutput, info: &OutputInfo, queue_handle: &QueueHandle<State>) -> Option<Self> {
        let name = info.name.as_deref().unwrap_or("unknown");
        let description = info.description.as_deref().unwrap_or("unknown");
        let scale = info.scale_factor.max(1).cast_unsigned();

        let Some((width, height)) = info.logical_size.filter(|(w, h)| *w > 0 && *h > 0).or_else(|| {
            info.modes.iter().find(|mode| mode.current).map(|mode| {
                let scale = info.scale_factor.max(1);
                ((mode.dimensions.0 / scale).max(1), (mode.dimensions.1 / scale).max(1))
            })
        }) else {
            tracing::warn!(name, "no valid dimensions, skipping output");
            return None;
        };

        let layer_surface = layer_shell.create_layer_surface(queue_handle, compositor.create_surface(queue_handle), Layer::Background, Some(env!("CARGO_PKG_NAME")), Some(&output));
        layer_surface.set_anchor(Anchor::all());
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_size(0, 0);

        if layer_surface.set_buffer_scale(scale).is_err() {
            tracing::warn!(name, scale, "compositor does not support buffer scaling, rendering at 1x");
        }
        layer_surface.commit();

        tracing::info!(name, description, width, height, scale, "monitor detected, creating wallpaper surface");
        Some(Self { layer_surface, output, output_name: name.to_string(), width: width.cast_unsigned(), height: height.cast_unsigned(), scale, status: Status::Unconfigured })
    }

    pub(super) fn needs_render(&self) -> bool {
        matches!(self.status, Status::NeedsRender)
    }

    pub(super) fn configure(&mut self, (new_width, new_height): (u32, u32)) {
        let resized = new_width > 0 && new_height > 0 && (new_width != self.width || new_height != self.height);
        if resized {
            tracing::info!(old_width = self.width, old_height = self.height, new_width, new_height, "compositor requested new surface size, resizing");
            (self.width, self.height) = (new_width, new_height);
        }

        if resized || matches!(self.status, Status::Unconfigured) {
            self.status = Status::NeedsRender;
        }
    }

    pub(super) fn rescale(&mut self, factor: i32) {
        let scale = factor.max(1).cast_unsigned();
        if scale == self.scale {
            return;
        }

        match self.layer_surface.set_buffer_scale(scale) {
            Ok(()) => {
                tracing::info!(old_scale = self.scale, scale, "output scale changed, resizing");
                self.scale = scale;
            }
            Err(e) => tracing::warn!(?e, "compositor does not support buffer scaling, keeping current resolution"),
        }

        self.status = Status::NeedsRender;
    }

    pub(super) fn start_transition(&mut self, image: &Image, config: &Config, shm: &Shm, queue_handle: &QueueHandle<State>) -> Result<()> {
        let (width, height) = (self.width.saturating_mul(self.scale), self.height.saturating_mul(self.scale));
        let size = width.saturating_mul(height).saturating_mul(4) as usize;

        let mut target = vec![0u8; size];
        image.render(width, height, &mut target, &config.resize)?;

        let color = config.transition.transition_color;
        let mut pixels = vec![0u8; size];
        pixels.chunks_exact_mut(4).for_each(|px| px.copy_from_slice(&[color.b, color.g, color.r, 0xFF]));

        self.status =
            Status::Transitioning { transition: Box::new(Transition::new(&config.transition, (width, height), target)), pool: SlotPool::new(size, shm).context("failed to create shm pool")?, pixels };

        self.tick(queue_handle)
    }

    pub(super) fn tick(&mut self, queue_handle: &QueueHandle<State>) -> Result<()> {
        let Status::Transitioning { transition, pool, pixels } = &mut self.status else {
            return Ok(());
        };

        let done = transition.frame(pixels);
        let (width, height) = (self.width.saturating_mul(self.scale).cast_signed(), self.height.saturating_mul(self.scale).cast_signed());
        let (buffer, canvas) = pool.create_buffer(width, height, width.saturating_mul(4), Format::Xrgb8888).context("failed to create buffer")?;
        canvas.copy_from_slice(pixels);

        let wl_surface = self.layer_surface.wl_surface();
        wl_surface.frame(queue_handle, FrameCallbackData(wl_surface.clone()));
        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, width, height);
        self.layer_surface.commit();

        if done {
            self.status = Status::Complete;
        }

        Ok(())
    }

    pub(super) fn destroy(&self) {
        tracing::info!(name = %self.output_name, "wallpaper surface destroyed for disconnected output");

        if let SurfaceKind::Wlr(layer_surface) = self.layer_surface.kind() {
            layer_surface.destroy();
        }

        self.layer_surface.wl_surface().destroy();
    }
}
