use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::Format;

use super::state::State;
use crate::config::Config;
use crate::image::Image;
use crate::transition::Transition;

enum SurfaceStatus {
    Pending,
    Ready,
    Transitioning { transition: Box<Transition>, pool: SlotPool, pixels: Vec<u8> },
    Idle,
}

pub(super) struct Surface {
    pub(super) layer_surface: LayerSurface,
    pub(super) output: WlOutput,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) output_name: String,
    status: SurfaceStatus,
}

impl Surface {
    pub(super) fn new(layer_surface: LayerSurface, output: WlOutput, width: u32, height: u32, output_name: String) -> Self {
        Self { layer_surface, output, width, height, output_name, status: SurfaceStatus::Pending }
    }

    pub(super) fn is_ready(&self) -> bool {
        matches!(self.status, SurfaceStatus::Ready)
    }

    pub(super) fn configure(&mut self) {
        if matches!(self.status, SurfaceStatus::Pending) {
            self.status = SurfaceStatus::Ready;
        }
    }

    pub(super) fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.status = SurfaceStatus::Ready;
    }

    pub(super) fn begin_transition(&mut self, image: &Image, config: &Config, shm: &Shm) -> Result<()> {
        let size = self.buffer_size();

        let mut target = vec![0u8; size];
        image.render(self.width, self.height, &mut target, &config.resize)?;

        let mut pixels = vec![0u8; size];
        let c = config.transition.transition_color;
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[c.b, c.g, c.r, 0xFF]);
        }

        let pool = SlotPool::new(size, shm).context("failed to create shm pool")?;
        let transition = Transition::new(&config.transition, (self.width, self.height), target);
        self.status = SurfaceStatus::Transitioning { transition: Box::new(transition), pool, pixels };

        Ok(())
    }

    pub(super) fn tick_transition(&mut self, queue_handle: &QueueHandle<State>) -> Result<()> {
        let done = match &mut self.status {
            SurfaceStatus::Transitioning { transition, pixels, .. } => transition.frame(pixels),
            _ => return Ok(()),
        };

        self.commit(queue_handle)?;

        if done {
            self.status = SurfaceStatus::Idle;
        }

        Ok(())
    }

    pub(super) fn commit(&mut self, queue_handle: &QueueHandle<State>) -> Result<()> {
        let width = self.width.cast_signed();
        let height = self.height.cast_signed();
        let stride = self.width.saturating_mul(4).cast_signed();
        let wl_surface = self.layer_surface.wl_surface();

        let SurfaceStatus::Transitioning { pool, pixels, .. } = &mut self.status else {
            return Ok(());
        };

        let (buffer, canvas) = pool.create_buffer(width, height, stride, Format::Xrgb8888).context("failed to create buffer")?;
        canvas.copy_from_slice(pixels);

        wl_surface.frame(queue_handle, FrameCallbackData(wl_surface.clone()));
        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, width, height);
        self.layer_surface.commit();

        Ok(())
    }

    fn buffer_size(&self) -> usize {
        self.width.saturating_mul(self.height).saturating_mul(4) as usize
    }
}
