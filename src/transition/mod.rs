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

fn random_float(min: f32, max: f32) -> f32 {
    min + (max - min) * rand::random::<f32>()
}

pub(crate) struct TransitionParams {
    pub(crate) direction: f32,
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
    pub(crate) stripe_count: f32,
    pub(crate) angle: f32,
    pub(crate) cell_size: f32,
    pub(crate) smoothness: f32,
}

fn randomize_params(transition_type: &TransitionType, config: &TransitionConfig, _aspect_ratio: f32) -> TransitionParams {
    let mut params = TransitionParams {
        direction: config.wipe.direction,
        center_x: config.disc.center_x,
        center_y: config.disc.center_y,
        stripe_count: config.stripes.stripe_count,
        angle: config.stripes.angle,
        cell_size: config.honeycomb.cell_size,
        smoothness: config.edge_smoothness,
    };

    match transition_type {
        TransitionType::Wipe => {
            params.direction = random_float(0.0, 4.0).floor();
        }
        TransitionType::Disc => {
            params.center_x = random_float(0.2, 0.8);
            params.center_y = random_float(0.2, 0.8);
        }
        TransitionType::Stripes => {
            params.stripe_count = random_float(4.0, 24.0).round();
            params.angle = random_float(0.0, 360.0);
        }
        TransitionType::Honeycomb => {
            params.cell_size = random_float(0.02, 0.06);
            params.center_x = random_float(0.2, 0.8);
            params.center_y = random_float(0.2, 0.8);
        }
        _ => {}
    }

    params
}

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
    fn new(config: &TransitionConfig, dimensions: (u32, u32)) -> Self {
        let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;
        let params = randomize_params(&config.transition_type, config, aspect_ratio);

        match config.transition_type {
            TransitionType::None => Self::None,
            TransitionType::Simple => Self::Cleanup { step: 2 },
            TransitionType::Fade => Self::Fade(Fade::new(config.duration)),
            TransitionType::Wipe => Self::Wipe(Wipe::new(config.duration, params.direction, params.smoothness, dimensions)),
            TransitionType::Disc => Self::Disc(Disc::new(config.duration, params.center_x, params.center_y, params.smoothness, dimensions)),
            TransitionType::Stripes => Self::Stripes(Stripes::new(config.duration, params.stripe_count, params.angle, params.smoothness, dimensions)),
            TransitionType::Zoom => Self::Zoom(Zoom::new(config.duration, dimensions)),
            TransitionType::Honeycomb => Self::Honeycomb(Honeycomb::new(config.duration, params.cell_size, params.center_x, params.center_y, params.smoothness, dimensions)),
        }
    }

    fn execute(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        match self {
            Self::None => {
                canvas.copy_from_slice(target);
                return true;
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
                return done;
            }
            Self::Fade(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: 4 };
            }
            Self::Wipe(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: 4 };
            }
            Self::Disc(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: 4 };
            }
            Self::Stripes(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: 4 };
            }
            Self::Zoom(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: 4 };
            }
            Self::Honeycomb(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: 4 };
            }
        }
        false
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
        let done = effect.execute(canvas, &self.target, elapsed);
        if done {
            tracing::info!(elapsed_secs = elapsed, duration = elapsed, "transition finished");
            self.effect = None;
            self.target = Vec::new();
        }
        done
    }
}
