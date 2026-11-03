use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm::Format;

use super::state::State;
use crate::config::Config;
use crate::image::Image;
use crate::transition::Transition;

pub(super) struct Frame {
    transition: Transition,
    pool: SlotPool,
    width: i32,
    height: i32,
    stride_bytes: i32,
}

impl Frame {
    pub(super) fn new(image: &Image, config: &Config, shm: &Shm, buffer_width: u32, buffer_height: u32) -> Result<Self> {
        if buffer_width == 0 || buffer_height == 0 {
            anyhow::bail!("frame dimensions must be non-zero");
        }

        if buffer_width > i32::MAX as u32 || buffer_height > i32::MAX as u32 {
            anyhow::bail!("frame dimensions too large");
        }

        let buffer_size = buffer_width.checked_mul(buffer_height).and_then(|pixels| pixels.checked_mul(4)).context("frame dimensions too large")?;
        if buffer_size > i32::MAX as u32 {
            anyhow::bail!("frame buffer exceeds wl_shm limits");
        }

        let rendered = image.render(buffer_width, buffer_height, &config.resize)?;
        if rendered.len() != buffer_size as usize {
            anyhow::bail!("rendered image size mismatch");
        }

        let pool = SlotPool::new(rendered.len(), shm).context("failed to create shm pool")?;
        let width = i32::try_from(buffer_width).context("width exceeds buffer limits")?;
        let height = i32::try_from(buffer_height).context("height exceeds buffer limits")?;
        let stride_bytes = width.checked_mul(4).context("stride overflow")?;

        Ok(Self { transition: Transition::new(&config.transition, (buffer_width, buffer_height), rendered), pool, width, height, stride_bytes })
    }

    pub(super) fn advance(&mut self, layer: &LayerSurface, qh: &QueueHandle<State>) -> Result<bool> {
        let (width, height, stride_bytes) = (self.width, self.height, self.stride_bytes);
        let (buffer, canvas) = self.pool.create_buffer(width, height, stride_bytes, Format::Xrgb8888).context("failed to create buffer")?;

        let row_count = usize::try_from(height).context("negative frame height")?;
        let stride_len = usize::try_from(stride_bytes).context("negative stride")?;
        let buffer_len = row_count.checked_mul(stride_len).context("buffer size overflow")?;
        let canvas = canvas.get_mut(..buffer_len).context("shm slot too small")?;

        let finished = self.transition.frame(canvas);
        let wl_surface = layer.wl_surface();
        wl_surface.damage_buffer(0, 0, width, height);
        if !finished {
            wl_surface.frame(qh, FrameCallbackData(wl_surface.clone()));
        }

        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        layer.commit();

        if finished {
            // SAFETY: malloc_trim only releases unused heap pages back to the OS and never
            // invalidates live allocations.
            #[cfg(all(target_os = "linux", target_env = "gnu"))]
            unsafe {
                libc::malloc_trim(0);
            }
        }

        Ok(finished)
    }
}
