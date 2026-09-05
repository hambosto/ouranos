mod effect;
mod math;
mod surface;

use std::time::Instant;

use effect::{Disc, Effect, Fade, Honeycomb, Stripes, Wipe, Zoom};
use surface::Surface;

use crate::config::{TransitionConfig, TransitionType};

enum Phase {
    Blit,
    Running { effect: Box<dyn Effect>, duration: f32 },
}

impl Phase {
    fn select(config: &TransitionConfig, dimensions: (u32, u32), surface: &Surface) -> Self {
        let duration = config.duration;
        let smoothness = config.edge_smoothness;
        let animated = |effect: Box<dyn Effect>| Self::Running { effect, duration };

        match config.transition_type {
            TransitionType::None | TransitionType::Simple => Self::Blit,
            TransitionType::Fade => animated(Box::new(Fade)),
            TransitionType::Wipe => animated(Box::new(Wipe::new(config.wipe.direction, smoothness))),
            TransitionType::Disc => animated(Box::new(Disc::new(config.disc.center_x, config.disc.center_y, smoothness, surface))),
            TransitionType::Stripes => animated(Box::new(Stripes::new(config.stripes.stripe_count, config.stripes.angle, smoothness, surface))),
            TransitionType::Zoom => animated(Box::new(Zoom::new(dimensions))),
            TransitionType::Honeycomb => animated(Box::new(Honeycomb::new(config.honeycomb.cell_size, config.honeycomb.center_x, config.honeycomb.center_y, smoothness, surface))),
        }
    }
}

pub(crate) struct Transition {
    phase: Phase,
    surface: Surface,
    target: Vec<u8>,
    start: Instant,
}

impl Transition {
    pub(crate) fn new(config: &TransitionConfig, dimensions: (u32, u32), target: Vec<u8>) -> Self {
        tracing::info!(
            transition_type = ?config.transition_type,
            duration = config.duration,
            width = dimensions.0,
            height = dimensions.1,
            pixels = dimensions.0 * dimensions.1,
            smoothness = config.edge_smoothness,
            "applying transition effect"
        );

        let surface = Surface::new(dimensions);
        Self { phase: Phase::select(config, dimensions, &surface), surface, target, start: Instant::now() }
    }

    pub(crate) fn frame(&self, canvas: &mut [u8]) -> bool {
        let elapsed = self.start.elapsed().as_secs_f64();

        let done = match &self.phase {
            Phase::Blit => {
                canvas.copy_from_slice(&self.target);
                true
            }
            Phase::Running { effect, duration } => {
                effect.render(&self.surface, canvas, &self.target, math::progress(*duration, elapsed));
                elapsed >= f64::from(*duration) && converge(canvas, &self.target)
            }
        };

        if done {
            tracing::info!(elapsed_secs = elapsed, "transition finished");
        }

        done
    }
}

fn converge(canvas: &mut [u8], target: &[u8]) -> bool {
    debug_assert_eq!(canvas.len(), target.len());
    let mut converged = true;

    for (old, new) in canvas.chunks_exact_mut(4).zip(target.chunks_exact(4)) {
        for (o, &n) in old.iter_mut().zip(new) {
            let delta = 4.min(o.abs_diff(n));
            *o = if *o > n { *o - delta } else { *o + delta };
        }
        converged &= old == new;
    }

    converged
}
