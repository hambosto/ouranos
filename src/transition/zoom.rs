use super::effect::Effect;
use super::surface::{Surface, sample_bilinear};

const STRENGTH: f32 = 0.15;

pub(crate) struct Zoom {
    stride: usize,
    extent: (f32, f32),
}

impl Zoom {
    pub(crate) fn new(surface: &Surface) -> Self {
        Self { stride: surface.stride(), extent: ((surface.width() - 1) as f32, (surface.height() - 1) as f32) }
    }
}

impl Effect for Zoom {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        let zoom_in_inv = 1.0 / (1.0 + STRENGTH * progress);
        let zoom_out_inv = 1.0 / (1.0 + STRENGTH * (1.0 - progress));
        let keep_old = 1.0 - progress;
        let stride = self.stride;
        let (max_x, max_y) = self.extent;

        let sample = |scale_inv: f32, u: f32, v: f32| {
            let x = ((u - 0.5) * scale_inv + 0.5) * max_x;
            let y = ((v - 0.5) * scale_inv + 0.5) * max_y;
            sample_bilinear(target, stride, x, y)
        };

        surface.for_each_pixel(canvas, target, |u, v, old_px, _| {
            let incoming = sample(zoom_in_inv, u, v);
            let outgoing = sample(zoom_out_inv, u, v);
            for ((o, &incoming), &outgoing) in old_px.iter_mut().zip(&incoming).zip(&outgoing) {
                let sampled = incoming * keep_old + outgoing * progress;
                *o = (*o as f32 * keep_old + sampled * progress + 0.5) as u8;
            }
        });
    }
}
