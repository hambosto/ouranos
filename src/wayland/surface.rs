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
    pixels: Vec<u8>,
    pool: Option<SlotPool>,
    pub(super) transition: Option<Transition>,
    pub(super) configured: bool,
    pub(super) rendered: bool,
}

impl Surface {
    pub(super) fn new(layer_surface: LayerSurface, width: u32, height: u32) -> Self {
        Self { layer_surface, width, height, pixels: Vec::new(), pool: None, transition: None, configured: false, rendered: false }
    }

    pub(super) fn buffer_size(&self) -> usize {
        self.width.saturating_mul(self.height).saturating_mul(4) as usize
    }

    pub(super) fn begin_transition(&mut self, image: &Image, config: &Config) -> Result<()> {
        let buffer_size = self.buffer_size();

        let mut target = vec![0u8; buffer_size];
        image.render(self.width, self.height, &mut target, &config.resize)?;

        let transition_color = config.transition.transition_color;
        self.pixels = Vec::with_capacity(buffer_size);
        for _ in 0..buffer_size / 4 {
            self.pixels.extend_from_slice(&[transition_color.b, transition_color.g, transition_color.r, 0xFF]);
        }
        self.transition = Some(Transition::new(&config.transition, (self.width, self.height), target));

        Ok(())
    }

    pub(super) fn tick(&mut self) {
        let Some(transition) = &mut self.transition else {
            return;
        };

        if transition.frame(&mut self.pixels) {
            self.transition = None;
        }
    }

    pub(super) fn commit(&mut self, shm: &Shm, queue_handle: &QueueHandle<State>) -> Result<()> {
        if self.pool.is_none() {
            self.pool = Some(SlotPool::new(self.buffer_size(), shm).context("failed to allocate shm pool")?);
        }

        let pool = self.pool.as_mut().context("shm pool not initialized")?;
        let width = self.width.cast_signed();
        let height = self.height.cast_signed();
        let stride = self.width.saturating_mul(4).cast_signed();

        let (buffer, canvas) = pool.create_buffer(width, height, stride, Format::Xrgb8888).context("failed to create buffer")?;
        canvas.copy_from_slice(&self.pixels);

        let wl_surface = self.layer_surface.wl_surface();
        if self.transition.is_some() {
            wl_surface.frame(queue_handle, FrameCallbackData(wl_surface.clone()));
        }

        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, width, height);
        self.layer_surface.commit();

        if self.transition.is_none() {
            self.pixels = Vec::new();
            self.pool = None;
        }

        Ok(())
    }
}
