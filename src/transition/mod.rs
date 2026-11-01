mod effect;
mod math;
mod surface;

use std::time::Instant;

use effect::Effect;
use surface::Surface;

use crate::config::TransitionConfig;

pub(crate) struct Transition {
    effect: Effect,
    surface: Surface,
    target: Vec<u8>,
    from: [u8; 4],
    duration: f32,
    start: Instant,
}

impl Transition {
    pub(crate) fn new(config: &TransitionConfig, dimensions: (u32, u32), target: Vec<u8>) -> Self {
        let expected = (u64::from(dimensions.0) * u64::from(dimensions.1) * 4) as usize;
        assert_eq!(target.len(), expected, "target buffer size mismatch for transition dimensions");

        let duration = if config.duration.is_finite() && config.duration >= 0.0 { config.duration } else { 1.5 };
        let smoothness = if config.edge_smoothness.is_finite() { config.edge_smoothness.clamp(0.0, 1.0) } else { 0.3 };

        tracing::info!(
            transition_type = ?config.transition_type,
            duration = duration,
            width = dimensions.0,
            height = dimensions.1,
            pixels = u64::from(dimensions.0) * u64::from(dimensions.1),
            smoothness = smoothness,
            "applying transition effect"
        );

        let surface = Surface::new(dimensions);
        let color = config.transition_color;

        Self { effect: Effect::select(config.transition_type, smoothness, &surface, dimensions), surface, target, from: [color.b, color.g, color.r, 0xFF], duration, start: Instant::now() }
    }

    pub(crate) fn frame(&self, canvas: &mut [u8]) -> bool {
        let elapsed = self.start.elapsed().as_secs_f64();
        if matches!(self.effect, Effect::Snap) || elapsed >= f64::from(self.duration) {
            canvas.copy_from_slice(&self.target);
            tracing::info!(elapsed_secs = elapsed, "transition finished");
            return true;
        }

        self.effect.render(&self.surface, canvas, &self.target, self.from, math::progress(self.duration, elapsed));

        false
    }
}
