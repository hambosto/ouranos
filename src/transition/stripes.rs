use super::animation::AnimationSequence;

pub(crate) struct Stripes {
    seq: AnimationSequence,
    stripe_count: f32,
    angle: f32,
    smoothness: f32,
    aspect_ratio: f32,
    width: usize,
    height: usize,
}

impl Stripes {
    pub(crate) fn new(duration: f32, stripe_count: f32, angle: f32, smoothness: f32, dimensions: (u32, u32)) -> Self {
        let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;
        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), stripe_count, angle, smoothness, aspect_ratio, width: dimensions.0 as usize, height: dimensions.1 as usize }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let mapped_smoothness = 0.001 + 0.299 * self.smoothness * self.smoothness;

        let rad = self.angle.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();

        let max_stripe_coord = self.aspect_ratio * cos_a.abs() + sin_a.abs();
        let stripe_width = max_stripe_coord / self.stripe_count;
        let max_perp = self.aspect_ratio * sin_a.abs() + cos_a.abs();

        let row_stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 / self.width as f32;
                let aspect_uv = (uv_x * self.aspect_ratio, uv_y);

                let stripe_coord = aspect_uv.0 * cos_a + aspect_uv.1 * sin_a;
                let perp_coord = -aspect_uv.0 * sin_a + aspect_uv.1 * cos_a;

                let stripe_pos = stripe_coord / stripe_width;
                let stripe_index = stripe_pos.floor();
                let is_odd = (stripe_index as i32 % 2) != 0;

                let normalized_perp = perp_coord / max_perp;
                let delay = normalized_perp.abs() * 0.3;
                let local_progress = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
                let local_frac = stripe_pos - stripe_index;

                let (edge_start, edge_end) = if is_odd { (1.0 + mapped_smoothness, -mapped_smoothness) } else { (-mapped_smoothness, 1.0 + mapped_smoothness) };
                let edge = edge_start * (1.0 - local_progress) + edge_end * local_progress;
                let factor = smoothstep(edge - mapped_smoothness, edge + mapped_smoothness, local_frac);

                let (f1, f2) = if is_odd { (1.0 - factor, factor) } else { (factor, 1.0 - factor) };

                for (c, &t) in canvas_px.iter_mut().zip(target_px) {
                    *c = (*c as f32 * f1 + t as f32 * f2 + 0.5) as u8;
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
