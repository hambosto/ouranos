use rand::{RngExt, rng};

use super::effect::Effect;
use super::math::{band_width, pick, smooth_edge};
use super::surface::Surface;

pub(crate) struct Wipe {
    along_x: bool,
    reversed: bool,
    band: f32,
}

impl Wipe {
    pub(crate) fn new(direction: f32, smoothness: f32) -> Self {
        let direction = pick(direction, 0.0, || rng().random_range(0.0..4.0)) as u32;
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
