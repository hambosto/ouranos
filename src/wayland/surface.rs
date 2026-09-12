use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::{CompositorState, FrameCallbackData};
use smithay_client_toolkit::output::OutputInfo;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Slot, SlotPool};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::Format;

use super::state::State;
use crate::config::Config;
use crate::image::Image;
use crate::transition::Transition;

pub(super) enum Status {
    Unconfigured,
    Pending,
    Transitioning(Box<Animation>),
    Complete,
}

pub(super) struct Animation {
    transition: Transition,
    pool: SlotPool,
    slot: Slot,
    width: i32,
    height: i32,
    stride: i32,
}

impl Animation {
    fn new(image: &Image, config: &Config, shm: &Shm, width: u32, height: u32) -> Result<Self> {
        width.checked_mul(height).and_then(|pixels| pixels.checked_mul(4)).context("surface dimensions too large")?;
        let target = image.render(width, height, &config.resize)?;

        let mut pool = SlotPool::new(target.len(), shm).context("failed to create shm pool")?;
        let slot = pool.new_slot(target.len()).context("failed to allocate shm slot")?;
        let (width_signed, height_signed) = (width.cast_signed(), height.cast_signed());

        Ok(Self { transition: Transition::new(&config.transition, (width, height), target), pool, slot, width: width_signed, height: height_signed, stride: width_signed.saturating_mul(4) })
    }

    fn present(&mut self, layer_surface: &LayerSurface, queue_handle: &QueueHandle<State>) -> Result<bool> {
        let (width, height, stride) = (self.width, self.height, self.stride);

        let (buffer, canvas) = if self.slot.has_active_buffers() {
            let (buffer, canvas) = self.pool.create_buffer(width, height, stride, Format::Xrgb8888).context("failed to create buffer")?;
            let exact = (height as usize).saturating_mul(stride as usize);
            let canvas = canvas.get_mut(..exact).context("shm slot too small")?;
            (buffer, canvas)
        } else {
            let buffer = self.pool.create_buffer_in(&self.slot, width, height, stride, Format::Xrgb8888).context("failed to create buffer")?;
            let canvas = buffer.canvas(&mut self.pool).context("shm slot busy")?;
            (buffer, canvas)
        };

        let done = self.transition.frame(canvas);
        let wl_surface = layer_surface.wl_surface();

        wl_surface.frame(queue_handle, FrameCallbackData(wl_surface.clone()));
        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, width, height);
        layer_surface.commit();

        Ok(done)
    }
}

pub(super) struct Surface {
    pub(super) layer_surface: LayerSurface,
    pub(super) output: WlOutput,
    width: u32,
    height: u32,
    scale: u32,
    pub(super) status: Status,
}

impl Surface {
    pub(super) fn create(compositor: &CompositorState, layer_shell: &LayerShell, output: WlOutput, info: &OutputInfo, queue_handle: &QueueHandle<State>) -> Option<Self> {
        let name = info.name.as_deref().unwrap_or("unknown");
        let description = info.description.as_deref().unwrap_or("unknown");
        let scale_factor = info.scale_factor.max(1);

        let Some((width, height)) = info.logical_size.filter(|(w, h)| *w > 0 && *h > 0).or_else(|| {
            info.modes
                .iter()
                .find(|mode| mode.current)
                .map(|mode| ((mode.dimensions.0 / scale_factor).max(1), (mode.dimensions.1 / scale_factor).max(1)))
        }) else {
            tracing::warn!(name, "no valid dimensions, skipping output");
            return None;
        };

        let scale = scale_factor.cast_unsigned();
        let layer_surface = layer_shell.create_layer_surface(queue_handle, compositor.create_surface(queue_handle), Layer::Background, Some(env!("CARGO_PKG_NAME")), Some(&output));

        layer_surface.set_anchor(Anchor::all());
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_size(0, 0);

        let scale = if layer_surface.set_buffer_scale(scale).is_err() {
            tracing::warn!(name, scale, "compositor does not support buffer scaling, rendering at 1x");
            1
        } else {
            scale
        };
        layer_surface.commit();

        tracing::info!(name, description, width, height, scale, "monitor detected, creating wallpaper surface");

        Some(Self { layer_surface, output, width: width.cast_unsigned(), height: height.cast_unsigned(), scale, status: Status::Unconfigured })
    }

    pub(super) fn configure(&mut self, (width, height): (u32, u32)) {
        let resized = width > 0 && height > 0 && (width != self.width || height != self.height);
        if resized {
            tracing::info!(old_width = self.width, old_height = self.height, width, height, "compositor requested new surface size, resizing");
            (self.width, self.height) = (width, height);
            self.status = Status::Pending;
        } else if matches!(self.status, Status::Unconfigured) {
            self.status = Status::Pending;
        }
    }

    pub(super) fn rescale(&mut self, factor: i32) {
        let scale = factor.max(1).cast_unsigned();
        if scale == self.scale {
            return;
        }

        if let Err(e) = self.layer_surface.set_buffer_scale(scale) {
            tracing::warn!(?e, "compositor does not support buffer scaling, keeping current resolution");
            return;
        }

        tracing::info!(old_scale = self.scale, scale, "output scale changed, resizing");
        self.scale = scale;
        self.status = Status::Pending;
    }

    pub(super) fn start_transition(&mut self, image: &Image, config: &Config, shm: &Shm, queue_handle: &QueueHandle<State>) -> Result<()> {
        let animation = Animation::new(image, config, shm, self.width.saturating_mul(self.scale), self.height.saturating_mul(self.scale))?;
        self.status = Status::Transitioning(Box::new(animation));
        self.tick(queue_handle)
    }

    pub(super) fn tick(&mut self, queue_handle: &QueueHandle<State>) -> Result<()> {
        let Status::Transitioning(animation) = &mut self.status else {
            return Ok(());
        };

        let done = match animation.present(&self.layer_surface, queue_handle) {
            Ok(done) => done,
            Err(e) => {
                self.status = Status::Pending;
                return Err(e);
            }
        };

        if done {
            self.status = Status::Complete;
            // SAFETY: malloc_trim only releases unused heap pages back to the OS and never
            // invalidates live allocations.
            unsafe {
                libc::malloc_trim(0);
            }
        }

        Ok(())
    }
}
