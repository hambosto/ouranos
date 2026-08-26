use rand::{RngExt, rng};

use super::effect::Effect;
use super::math::{band_width, dist, max_corner_distance, pick, random_center, smooth_edge};
use super::surface::Surface;

pub(crate) struct Honeycomb {
    cell_size: f32,
    origin: (f32, f32),
    band: f32,
    radius_max: f32,
    inv_cell_size: f32,
    sqrt3: f32,
    aspect_ratio: f32,
}

impl Honeycomb {
    pub(crate) fn new(cell_size: f32, center_x: f32, center_y: f32, smoothness: f32, surface: &Surface) -> Self {
        let aspect_ratio = surface.aspect_ratio();
        let origin = (pick(center_x, 0.5, random_center) * aspect_ratio, pick(center_y, 0.5, random_center));
        let cell_size = pick(cell_size, 0.04, || rng().random_range(0.02..0.06));
        let band = band_width(smoothness, 0.499);

        Self { cell_size, origin, band, radius_max: max_corner_distance(origin, aspect_ratio) + 2.0 * band, inv_cell_size: 1.0 / cell_size, sqrt3: 3.0_f32.sqrt(), aspect_ratio }
    }
}

impl Effect for Honeycomb {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        let radius = progress * self.radius_max - self.band;

        surface.blend(canvas, target, |u, v| {
            let sqrt3 = self.sqrt3;
            let x = u * self.aspect_ratio;

            let q = x * (2.0 / 3.0) * self.inv_cell_size;
            let r = (-x / 3.0 + sqrt3 / 3.0 * v) * self.inv_cell_size;

            let y = -q - r;
            let rx = (q + 0.5).floor();
            let rz = (r + 0.5).floor();
            let ry = (y + 0.5).floor();

            let (dx, dy, dz) = ((rx - q).abs(), (ry - y).abs(), (rz - r).abs());

            let (hex_q, hex_r) = if dx > dy && dx > dz {
                (-ry - rz, rz)
            } else if dy > dz {
                (rx, rz)
            } else {
                (rx, -rx - ry)
            };

            let hex_x = hex_q * 1.5 * self.cell_size;
            let hex_y = (hex_q * sqrt3 / 2.0 + hex_r * sqrt3) * self.cell_size;
            let distance = dist(self.origin, (hex_x, hex_y));

            smooth_edge(distance, radius, self.band)
        });
    }
}
