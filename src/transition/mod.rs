pub(crate) mod animation;
mod disc;
mod fade;
mod honeycomb;
mod stripes;
mod wipe;
mod zoom;

use std::time::Instant;

use disc::Disc;
use fade::Fade;
use honeycomb::Honeycomb;
use stripes::Stripes;
use wipe::Wipe;
use zoom::Zoom;

use crate::config::{TransitionConfig, TransitionType};

enum Effect {
    None,
    Cleanup { step: u8 },
    Fade(Fade),
    Wipe(Wipe),
    Disc(Disc),
    Stripes(Stripes),
    Zoom(Zoom),
    Honeycomb(Honeycomb),
}

impl Effect {
    fn new(config: &TransitionConfig, (w, h): (u32, u32)) -> Self {
        let dur = config.duration;
        let smooth = config.edge_smoothness;
        let dims = (w, h);

        match config.transition_type {
            TransitionType::None => Self::None,
            TransitionType::Simple => Self::Cleanup { step: 2 },
            TransitionType::Fade => Self::Fade(Fade::new(dur)),
            TransitionType::Wipe => {
                let dir = if config.wipe.direction == 0.0 { (rand::random::<f32>() * 4.0).floor() } else { config.wipe.direction };
                Self::Wipe(Wipe::new(dur, dir, smooth, dims))
            }
            TransitionType::Disc => {
                let rand_xy = || 0.2 + rand::random::<f32>() * 0.6;
                let cx = if config.disc.center_x == 0.5 { rand_xy() } else { config.disc.center_x };
                let cy = if config.disc.center_y == 0.5 { rand_xy() } else { config.disc.center_y };
                Self::Disc(Disc::new(dur, cx, cy, smooth, dims))
            }
            TransitionType::Stripes => {
                let count = if config.stripes.stripe_count == 12.0 {
                    (4.0 + rand::random::<f32>() * 20.0).round()
                } else {
                    config.stripes.stripe_count
                };
                let angle = if config.stripes.angle == 30.0 { rand::random::<f32>() * 360.0 } else { config.stripes.angle };
                Self::Stripes(Stripes::new(dur, count, angle, smooth, dims))
            }
            TransitionType::Zoom => Self::Zoom(Zoom::new(dur, dims)),
            TransitionType::Honeycomb => {
                let rand_xy = || 0.2 + rand::random::<f32>() * 0.6;
                let size = if config.honeycomb.cell_size == 0.04 { 0.02 + rand::random::<f32>() * 0.04 } else { config.honeycomb.cell_size };
                let cx = if config.honeycomb.center_x == 0.5 { rand_xy() } else { config.honeycomb.center_x };
                let cy = if config.honeycomb.center_y == 0.5 { rand_xy() } else { config.honeycomb.center_y };
                Self::Honeycomb(Honeycomb::new(dur, size, cx, cy, smooth, dims))
            }
        }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        match self {
            Self::None => {
                canvas.copy_from_slice(target);
                true
            }
            Self::Cleanup { step } => {
                let step = *step;
                let mut done = true;
                for (old, new) in canvas.chunks_exact_mut(4).zip(target.chunks_exact(4)) {
                    for (o, &n) in old.iter_mut().zip(new) {
                        let delta = step.min(o.abs_diff(n));
                        *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
                    }
                    done &= old == new;
                }
                done
            }
            effect => {
                let done = match effect {
                    Self::Fade(e) => e.run(canvas, target, elapsed),
                    Self::Wipe(e) => e.run(canvas, target, elapsed),
                    Self::Disc(e) => e.run(canvas, target, elapsed),
                    Self::Stripes(e) => e.run(canvas, target, elapsed),
                    Self::Zoom(e) => e.run(canvas, target, elapsed),
                    Self::Honeycomb(e) => e.run(canvas, target, elapsed),
                    _ => unreachable!(),
                };
                if done {
                    *effect = Self::Cleanup { step: 4 };
                }
                false
            }
        }
    }
}

pub(crate) struct Transition {
    effect: Option<Effect>,
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
        Self { effect: Some(Effect::new(config, dimensions)), target, start: Instant::now() }
    }

    pub(crate) fn frame(&mut self, canvas: &mut [u8]) -> bool {
        let Some(effect) = self.effect.as_mut() else {
            return true;
        };

        let elapsed = self.start.elapsed().as_secs_f64();
        let done = effect.run(canvas, &self.target, elapsed);

        if done {
            tracing::info!(elapsed_secs = elapsed, "transition finished");
            self.effect = None;
            self.target.clear();
        }
        done
    }
}
