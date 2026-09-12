use super::math::{band_width, dist, max_corner_distance, smooth_edge};
use super::surface::Surface;
use crate::config::TransitionType;

#[derive(Clone, Copy)]
pub(crate) enum Effect {
    Snap,
    Fade,
    Wipe { along_x: bool, reversed: bool, band: f32 },
    Disc { center: (f32, f32), band: f32, radius_max: f32 },
    Stripes { band: f32, stripe_width: f32, max_perpendicular: f32, cos_a: f32, sin_a: f32 },
    Honeycomb { cell_size: f32, origin: (f32, f32), band: f32, radius_max: f32 },
    Zoom { extent: (f32, f32) },
}

impl Effect {
    pub(crate) fn select(kind: TransitionType, smoothness: f32, surface: &Surface, dimensions: (u32, u32)) -> Self {
        match kind {
            TransitionType::None => Self::Snap,
            TransitionType::Fade => Self::Fade,
            TransitionType::Wipe => {
                let direction = rand::random_range(0..4_u32);
                Self::Wipe { along_x: matches!(direction, 0 | 1), reversed: matches!(direction, 0 | 2), band: band_width(smoothness, 0.499) }
            }
            TransitionType::Disc => {
                let aspect = surface.aspect_ratio();
                let center = (rand::random_range(0.2..0.8_f32) * aspect, rand::random_range(0.2..0.8_f32));
                let band = band_width(smoothness, 0.499);
                Self::Disc { center, band, radius_max: max_corner_distance(center, aspect) + 2.0 * band }
            }
            TransitionType::Stripes => {
                let aspect = surface.aspect_ratio();
                let (sin_a, cos_a) = rand::random_range(0.0..360.0_f32).to_radians().sin_cos();
                let count = rand::random_range(4.0..24.0_f32).round();
                Self::Stripes { band: band_width(smoothness, 0.299), stripe_width: (aspect * cos_a.abs() + sin_a.abs()) / count, max_perpendicular: aspect * sin_a.abs() + cos_a.abs(), cos_a, sin_a }
            }
            TransitionType::Zoom => Self::Zoom { extent: (dimensions.0.saturating_sub(1) as f32, dimensions.1.saturating_sub(1) as f32) },
            TransitionType::Honeycomb => {
                let aspect = surface.aspect_ratio();
                let origin = (rand::random_range(0.2..0.8_f32) * aspect, rand::random_range(0.2..0.8_f32));
                let band = band_width(smoothness, 0.499);
                Self::Honeycomb { cell_size: rand::random_range(0.02..0.06_f32), origin, band, radius_max: max_corner_distance(origin, aspect) + 2.0 * band }
            }
        }
    }

    pub(crate) fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], from: [u8; 4], progress: f32) {
        match *self {
            Self::Snap => canvas.copy_from_slice(target),
            Self::Fade => surface.blend(canvas, target, from, |_, _| 1.0 - progress),
            Self::Wipe { along_x, reversed, band } => {
                let travel = progress * (1.0 + 2.0 * band) - band;
                let edge = if reversed { 1.0 - travel } else { travel };
                surface.blend(canvas, target, from, |u, v| {
                    let coord = if along_x { u } else { v };
                    let factor = smooth_edge(coord, edge, band);
                    if reversed { 1.0 - factor } else { factor }
                });
            }
            Self::Disc { center, band, radius_max } => {
                let radius = progress * radius_max - band;
                let aspect = surface.aspect_ratio();
                surface.blend(canvas, target, from, |u, v| smooth_edge(dist(center, (u * aspect, v)), radius, band));
            }
            Self::Stripes { band, stripe_width, max_perpendicular, cos_a, sin_a } => {
                surface.blend(canvas, target, from, |u, v| {
                    let x = u * surface.aspect_ratio();
                    let along = (x * cos_a + v * sin_a) / stripe_width;
                    let perpendicular = (-x * sin_a + v * cos_a) / max_perpendicular;

                    let stripe = along.floor();
                    let fraction = along - stripe;
                    let odd = (stripe as i32 % 2) != 0;

                    let delay = perpendicular.abs() * 0.3;
                    let local = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);

                    let (from, to) = if odd { (1.0 + band, -band) } else { (-band, 1.0 + band) };
                    let edge = from * (1.0 - local) + to * local;

                    let factor = smooth_edge(fraction, edge, band);
                    if odd { 1.0 - factor } else { factor }
                });
            }
            Self::Honeycomb { cell_size, origin, band, radius_max } => {
                let radius = progress * radius_max - band;
                let sqrt3 = 3.0_f32.sqrt();
                let inv_cell_size = 1.0 / cell_size;

                surface.blend(canvas, target, from, |u, v| {
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

                    let hex_x = hex_q * 1.5 * cell_size;
                    let hex_y = (hex_q * sqrt3 / 2.0 + hex_r * sqrt3) * cell_size;

                    smooth_edge(dist(origin, (hex_x, hex_y)), radius, band)
                });
            }
            Self::Zoom { extent } => {
                let inv = 1.0 / (1.0 + 0.15 * (1.0 - progress));
                let keep = 1.0 - progress;
                let (max_x, max_y) = extent;

                surface.for_each_pixel(canvas, target, |u, v, px, _| {
                    let sampled = surface.sample(target, ((u - 0.5) * inv + 0.5) * max_x, ((v - 0.5) * inv + 0.5) * max_y);
                    for ((o, &a), &s) in px.iter_mut().zip(&from).zip(&sampled) {
                        *o = (a as f32 * keep + s * progress + 0.5) as u8;
                    }
                });
            }
        }
    }
}
