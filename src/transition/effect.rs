use super::surface::Surface;

pub(crate) trait Effect {
    fn render(&self, surface: &Surface, canvas: &mut [u8], target: &[u8], progress: f32);
}
