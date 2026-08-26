mod disc;
mod effect;
mod fade;
mod honeycomb;
mod math;
mod stripes;
mod surface;
mod wipe;
mod zoom;

use std::time::Instant;

use crate::config::{TransitionConfig, TransitionType};
use disc::Disc;
use effect::Effect;
use fade::Fade;
use honeycomb::Honeycomb;
use stripes::Stripes;
use surface::Surface;
use wipe::Wipe;
use zoom::Zoom;

enum Phase {
    Blit,
    Running { effect: Box<dyn Effect>, duration: f32 },
    Snapping { step: u8 },
    Done,
}

impl Phase {
    fn select(config: &TransitionConfig, surface: &Surface) -> Self {
        let duration = config.duration;
        let smoothness = config.edge_smoothness;
        let animated = |effect: Box<dyn Effect>| Self::Running { effect, duration };

        match config.transition_type {
            TransitionType::None => Self::Blit,
            TransitionType::Simple => Self::Snapping { step: 2 },
            TransitionType::Fade => animated(Box::new(Fade)),
            TransitionType::Wipe => animated(Box::new(Wipe::new(config.wipe.direction, smoothness))),
            TransitionType::Disc => animated(Box::new(Disc::new(config.disc.center_x, config.disc.center_y, smoothness, surface))),
            TransitionType::Stripes => animated(Box::new(Stripes::new(config.stripes.stripe_count, config.stripes.angle, smoothness, surface))),
            TransitionType::Zoom => animated(Box::new(Zoom::new(surface))),
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
        Self { phase: Phase::select(config, &surface), surface, target, start: Instant::now() }
    }

    pub(crate) fn frame(&mut self, canvas: &mut [u8]) -> bool {
        let Self { phase, surface, target, start } = self;
        let elapsed = start.elapsed().as_secs_f64();

        let done = match phase {
            Phase::Done => true,
            Phase::Blit => {
                canvas.copy_from_slice(target);
                true
            }
            Phase::Snapping { step } => converge(canvas, target, *step),
            Phase::Running { effect, duration } => {
                effect.render(surface, canvas, target, math::progress(*duration, elapsed));
                if elapsed >= f64::from(*duration) {
                    *phase = Phase::Snapping { step: 4 };
                }
                false
            }
        };

        if done {
            tracing::info!(elapsed_secs = elapsed, "transition finished");
            self.phase = Phase::Done;
            self.target.clear();
        }
        done
    }
}

fn converge(canvas: &mut [u8], target: &[u8], step: u8) -> bool {
    let mut converged = true;

    for (old, new) in canvas.chunks_exact_mut(4).zip(target.chunks_exact(4)) {
        for (o, &n) in old.iter_mut().zip(new) {
            let delta = step.min(o.abs_diff(n));
            *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
        }
        converged &= old == new;
    }

    converged
}
