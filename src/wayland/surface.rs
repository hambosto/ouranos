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

pub(super) struct Surface {
    pub(super) layer_surface: LayerSurface,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) output_name: String,
    pub(super) configured: bool,
    pool: Option<SlotPool>,
    pixels: Vec<u8>,
    pub(super) transition: Option<Transition>,
}

impl Surface {
    pub(super) fn new(layer_surface: LayerSurface, width: u32, height: u32, output_name: String) -> Self {
        Self { layer_surface, width, height, output_name, configured: false, pool: None, pixels: Vec::new(), transition: None }
    }

    pub(super) fn configure(&mut self) {
        self.configured = true;
    }

    pub(super) fn begin_transition(&mut self, image: &Image, config: &Config, shm: &Shm) -> Result<()> {
        let size = self.buffer_size();

        let mut target = vec![0u8; size];
        image.render(self.width, self.height, &mut target, &config.resize)?;

        self.pixels = vec![0u8; size];
        let c = config.transition.transition_color;
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[c.b, c.g, c.r, 0xFF]);
        }

        self.pool = Some(SlotPool::new(size, shm).context("failed to create shm pool")?);
        self.transition = Some(Transition::new(&config.transition, (self.width, self.height), target));

        Ok(())
    }

    pub(super) fn tick_transition(&mut self, shm: &Shm, queue_handle: &QueueHandle<State>) -> Result<()> {
        let Some(transition) = &mut self.transition else {
            return Ok(());
        };

        let done = transition.frame(&mut self.pixels);
        self.commit(shm, queue_handle)?;

        if done {
            self.transition = None;
            self.pixels = Vec::new();
            self.pool = None;
        }

        Ok(())
    }

    pub(super) fn commit(&mut self, shm: &Shm, queue_handle: &QueueHandle<State>) -> Result<()> {
        if self.pool.is_none() {
            self.pool = Some(SlotPool::new(self.buffer_size(), shm).context("failed to create shm pool")?);
        }

        let pool = self.pool.as_mut().context("shm pool not initialized")?;
        let width = self.width.cast_signed();
        let height = self.height.cast_signed();
        let stride = self.width.saturating_mul(4).cast_signed();

        let (buffer, canvas) = pool.create_buffer(width, height, stride, Format::Xrgb8888).context("failed to create buffer")?;

        if !self.pixels.is_empty() {
            canvas.copy_from_slice(&self.pixels);
        }

        let wl_surface = self.layer_surface.wl_surface();
        if self.transition.is_some() {
            wl_surface.frame(queue_handle, FrameCallbackData(wl_surface.clone()));
        }

        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, width, height);
        self.layer_surface.commit();

        Ok(())
    }

    fn buffer_size(&self) -> usize {
        self.width.saturating_mul(self.height).saturating_mul(4) as usize
    }
}
