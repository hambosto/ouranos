use super::animation::AnimationSequence;

pub(crate) struct Disc {
    seq: AnimationSequence,
    center: (f32, f32),
    smoothness: f32,
    radius_max: f32,
    width: usize,
    height: usize,
    aspect_ratio: f32,
}

impl Disc {
    pub(crate) fn new(duration: f32, center_x: f32, center_y: f32, smoothness: f32, (w, h): (u32, u32)) -> Self {
        let aspect_ratio = w as f32 / h as f32;
        let center = (center_x * aspect_ratio, center_y);
        let s = 0.001 + 0.499 * smoothness * smoothness;

        let mut max_dist = 0.0_f32;
        for x in [0.0, aspect_ratio] {
            for y in [0.0, 1.0] {
                let dx = center.0 - x;
                let dy = center.1 - y;
                max_dist = max_dist.max((dx * dx + dy * dy).sqrt());
            }
        }

        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), center, smoothness: s, radius_max: max_dist + 2.0 * s, width: w as usize, height: h as usize, aspect_ratio }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let s = self.smoothness;
        let radius = progress * self.radius_max - s;
        let row_stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 / self.width as f32;
                let dx = self.center.0 - uv_x * self.aspect_ratio;
                let dy = self.center.1 - uv_y;
                let dist = (dx * dx + dy * dy).sqrt();

                let t = ((dist - radius + s) / (2.0 * s)).clamp(0.0, 1.0);
                let factor = t * t * (3.0 - 2.0 * t);

                let inv = 1.0 - factor;
                for (c, &t) in canvas_px.iter_mut().zip(target_px) {
                    *c = (t as f32 * inv + *c as f32 * factor + 0.5) as u8;
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}
