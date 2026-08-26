use rand::{RngExt, rng};

use super::effect::Effect;
use super::math::{band_width, pick, smooth_edge};
use super::surface::Surface;

pub(crate) struct Stripes {
    band: f32,
    stripe_width: f32,
    max_perpendicular: f32,
    cos_a: f32,
    sin_a: f32,
    aspect_ratio: f32,
}

impl Stripes {
    pub(crate) fn new(stripe_count: f32, angle: f32, smoothness: f32, surface: &Surface) -> Self {
        let stripe_count = pick(stripe_count, 12.0, || rng().random_range(4.0_f32..24.0).round());
        let angle = pick(angle, 30.0, || rng().random_range(0.0..360.0));
        let (sin_a, cos_a) = angle.to_radians().sin_cos();
        let aspect_ratio = surface.aspect_ratio();

        Self {
            band: band_width(smoothness, 0.299),
            stripe_width: (aspect_ratio * cos_a.abs() + sin_a.abs()) / stripe_count,
            max_perpendicular: aspect_ratio * sin_a.abs() + cos_a.abs(),
            cos_a,
            sin_a,
            aspect_ratio,
        }
    }
}

impl Effect for Stripes {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        surface.blend(canvas, target, |u, v| {
            let x = u * self.aspect_ratio;
            let along = (x * self.cos_a + v * self.sin_a) / self.stripe_width;
            let perpendicular = (-x * self.sin_a + v * self.cos_a) / self.max_perpendicular;

            let stripe = along.floor();
            let fraction = along - stripe;
            let odd = (stripe as i32 % 2) != 0;

            let delay = perpendicular.abs() * 0.3;
            let local = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);

            let (from, to) = if odd { (1.0 + self.band, -self.band) } else { (-self.band, 1.0 + self.band) };
            let edge = from * (1.0 - local) + to * local;

            let factor = smooth_edge(fraction, edge, self.band);
            if odd { 1.0 - factor } else { factor }
        });
    }
}
