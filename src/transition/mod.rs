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
        tracing::info!(
            transition_type = ?config.transition_type,
            duration = config.duration,
            width = dimensions.0,
            height = dimensions.1,
            pixels = u64::from(dimensions.0) * u64::from(dimensions.1),
            smoothness = config.edge_smoothness,
            "applying transition effect"
        );

        let surface = Surface::new(dimensions);
        let color = config.transition_color;

        Self {
            effect: Effect::select(config.transition_type, config.edge_smoothness, &surface, dimensions),
            surface,
            target,
            from: [color.b, color.g, color.r, 0xFF],
            duration: config.duration,
            start: Instant::now(),
        }
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
