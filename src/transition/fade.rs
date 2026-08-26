use super::effect::Effect;
use super::surface::Surface;

pub(crate) struct Fade;

impl Effect for Fade {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32) {
        surface.blend(canvas, target, |_, _| 1.0 - progress);
    }
}
