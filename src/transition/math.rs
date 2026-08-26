use rand::{RngExt, rng};

pub(crate) fn progress(duration: f32, elapsed: f64) -> f32 {
    if duration <= 0.0 {
        return 1.0;
    }
    ease_in_out_cubic((elapsed / f64::from(duration)).clamp(0.0, 1.0) as f32)
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub(crate) fn smooth_edge(value: f32, edge: f32, band: f32) -> f32 {
    smoothstep(((value - edge + band) / (2.0 * band)).clamp(0.0, 1.0))
}

pub(crate) fn band_width(smoothness: f32, scale: f32) -> f32 {
    0.001 + scale * smoothness * smoothness
}

pub(crate) fn max_corner_distance(origin: (f32, f32), aspect_ratio: f32) -> f32 {
    let mut max = 0.0_f32;
    for x in [0.0, aspect_ratio] {
        for y in [0.0, 1.0] {
            max = max.max(dist(origin, (x, y)));
        }
    }
    max
}

pub(crate) fn dist((ax, ay): (f32, f32), (bx, by): (f32, f32)) -> f32 {
    let (dx, dy) = (ax - bx, ay - by);
    (dx * dx + dy * dy).sqrt()
}

pub(crate) fn random_center() -> f32 {
    rng().random_range(0.2..0.8)
}

pub(crate) fn pick(value: f32, fallback: f32, generate: impl FnOnce() -> f32) -> f32 {
    if value == fallback { generate() } else { value }
}
