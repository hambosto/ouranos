use super::math::{band_width, dist, max_corner_distance, smooth_edge};
use super::surface::Surface;

pub(crate) trait Effect {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32);
}

pub(crate) struct Fade;

impl Effect for Fade {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        surface.blend(canvas, target, |_, _| 1.0 - progress);
    }
}

pub(crate) struct Wipe {
    along_x: bool,
    reversed: bool,
    band: f32,
}

impl Wipe {
    pub(crate) fn new(direction: f32, smoothness: f32) -> Self {
        let direction = direction as u32;
        Self { along_x: matches!(direction, 0 | 1), reversed: matches!(direction, 0 | 2), band: band_width(smoothness, 0.499) }
    }
}

impl Effect for Wipe {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        let travel = progress * (1.0 + 2.0 * self.band) - self.band;
        let edge = if self.reversed { 1.0 - travel } else { travel };
        surface.blend(canvas, target, |u, v| {
            let coord = if self.along_x { u } else { v };
            let factor = smooth_edge(coord, edge, self.band);
            if self.reversed { 1.0 - factor } else { factor }
        });
    }
}

pub(crate) struct Disc {
    center: (f32, f32),
    band: f32,
    radius_max: f32,
}

impl Disc {
    pub(crate) fn new(center_x: f32, center_y: f32, smoothness: f32, surface: &Surface) -> Self {
        let aspect_ratio = surface.aspect_ratio();
        let center = (center_x * aspect_ratio, center_y);
        let band = band_width(smoothness, 0.499);

        Self { center, band, radius_max: max_corner_distance(center, aspect_ratio) + 2.0 * band }
    }
}

impl Effect for Disc {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        let radius = progress * self.radius_max - self.band;

        surface.blend(canvas, target, |u, v| smooth_edge(dist(self.center, (u * surface.aspect_ratio(), v)), radius, self.band));
    }
}

pub(crate) struct Stripes {
    band: f32,
    stripe_width: f32,
    max_perpendicular: f32,
    cos_a: f32,
    sin_a: f32,
}

impl Stripes {
    pub(crate) fn new(stripe_count: f32, angle: f32, smoothness: f32, surface: &Surface) -> Self {
        let stripe_count = stripe_count.max(1.0);
        let (sin_a, cos_a) = angle.to_radians().sin_cos();
        let aspect_ratio = surface.aspect_ratio();

        Self { band: band_width(smoothness, 0.299), stripe_width: (aspect_ratio * cos_a.abs() + sin_a.abs()) / stripe_count, max_perpendicular: aspect_ratio * sin_a.abs() + cos_a.abs(), cos_a, sin_a }
    }
}

impl Effect for Stripes {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        surface.blend(canvas, target, |u, v| {
            let x = u * surface.aspect_ratio();
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

pub(crate) struct Honeycomb {
    cell_size: f32,
    origin: (f32, f32),
    band: f32,
    radius_max: f32,
}

impl Honeycomb {
    pub(crate) fn new(cell_size: f32, center_x: f32, center_y: f32, smoothness: f32, surface: &Surface) -> Self {
        let aspect_ratio = surface.aspect_ratio();
        let origin = (center_x * aspect_ratio, center_y);
        let cell_size = cell_size.max(1e-6);
        let band = band_width(smoothness, 0.499);

        Self { cell_size, origin, band, radius_max: max_corner_distance(origin, aspect_ratio) + 2.0 * band }
    }
}

impl Effect for Honeycomb {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        let radius = progress * self.radius_max - self.band;
        let sqrt3 = 3.0_f32.sqrt();
        let inv_cell_size = 1.0 / self.cell_size;

        surface.blend(canvas, target, |u, v| {
            let x = u * surface.aspect_ratio();

            let q = x * (2.0 / 3.0) * inv_cell_size;
            let r = (-x / 3.0 + sqrt3 / 3.0 * v) * inv_cell_size;

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

const ZOOM_STRENGTH: f32 = 0.15;

pub(crate) struct Zoom {
    extent: (f32, f32),
}

impl Zoom {
    pub(crate) fn new((width, height): (u32, u32)) -> Self {
        Self { extent: (width.saturating_sub(1) as f32, height.saturating_sub(1) as f32) }
    }
}

impl Effect for Zoom {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        let zoom_in_inv = 1.0 / (1.0 + ZOOM_STRENGTH * progress);
        let zoom_out_inv = 1.0 / (1.0 + ZOOM_STRENGTH * (1.0 - progress));
        let keep_old = 1.0 - progress;
        let (max_x, max_y) = self.extent;

        surface.for_each_pixel(canvas, target, |u, v, old_px, _| {
            let incoming = surface.sample(target, ((u - 0.5) * zoom_in_inv + 0.5) * max_x, ((v - 0.5) * zoom_in_inv + 0.5) * max_y);
            let outgoing = surface.sample(target, ((u - 0.5) * zoom_out_inv + 0.5) * max_x, ((v - 0.5) * zoom_out_inv + 0.5) * max_y);
            for ((o, &incoming), &outgoing) in old_px.iter_mut().zip(&incoming).zip(&outgoing) {
                let sampled = incoming * keep_old + outgoing * progress;
                *o = (*o as f32 * keep_old + sampled * progress + 0.5) as u8;
            }
        });
    }
}
