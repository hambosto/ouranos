use super::animation::AnimationSequence;

pub(crate) struct Stripes {
    seq: AnimationSequence,
    smoothness: f32,
    stripe_width: f32,
    max_perp: f32,
    cos_a: f32,
    sin_a: f32,
    width: usize,
    height: usize,
    aspect_ratio: f32,
}

impl Stripes {
    pub(crate) fn new(duration: f32, stripe_count: f32, angle: f32, smoothness: f32, (w, h): (u32, u32)) -> Self {
        let aspect_ratio = w as f32 / h as f32;
        let rad = angle.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();
        let max_stripe_coord = aspect_ratio * cos_a.abs() + sin_a.abs();

        Self {
            seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0),
            smoothness: 0.001 + 0.299 * smoothness * smoothness,
            stripe_width: max_stripe_coord / stripe_count,
            max_perp: aspect_ratio * sin_a.abs() + cos_a.abs(),
            cos_a,
            sin_a,
            width: w as usize,
            height: h as usize,
            aspect_ratio,
        }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let s = self.smoothness;
        let row_stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 / self.width as f32;
                let ax = uv_x * self.aspect_ratio;

                let stripe_coord = ax * self.cos_a + uv_y * self.sin_a;
                let perp_coord = -ax * self.sin_a + uv_y * self.cos_a;

                let stripe_pos = stripe_coord / self.stripe_width;
                let stripe_index = stripe_pos.floor();
                let is_odd = (stripe_index as i32 % 2) != 0;

                let delay = (perp_coord / self.max_perp).abs() * 0.3;
                let local_progress = ((progress - delay) / (1.0 - delay)).clamp(0.0, 1.0);
                let local_frac = stripe_pos - stripe_index;

                let (edge_start, edge_end) = if is_odd { (1.0 + s, -s) } else { (-s, 1.0 + s) };
                let edge = edge_start * (1.0 - local_progress) + edge_end * local_progress;

                let t = ((local_frac - edge + s) / (2.0 * s)).clamp(0.0, 1.0);
                let factor = t * t * (3.0 - 2.0 * t);

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
