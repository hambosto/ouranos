use super::effect::Effect;
use super::math::{band_width, dist, max_corner_distance, pick, random_center, smooth_edge};
use super::surface::Surface;

pub(crate) struct Disc {
    center: (f32, f32),
    band: f32,
    radius_max: f32,
    aspect_ratio: f32,
}

impl Disc {
    pub(crate) fn new(center_x: f32, center_y: f32, smoothness: f32, surface: &Surface) -> Self {
        let aspect_ratio = surface.aspect_ratio();
        let center = (pick(center_x, 0.5, random_center) * aspect_ratio, pick(center_y, 0.5, random_center));
        let band = band_width(smoothness, 0.499);

        Self { center, band, radius_max: max_corner_distance(center, aspect_ratio) + 2.0 * band, aspect_ratio }
    }
}

impl Effect for Disc {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        let radius = progress * self.radius_max - self.band;

        surface.blend(canvas, target, |u, v| smooth_edge(dist(self.center, (u * self.aspect_ratio, v)), radius, self.band));
    }
}
