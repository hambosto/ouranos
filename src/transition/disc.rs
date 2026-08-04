use super::animation::AnimationSequence;

pub(crate) struct Disc {
    seq: AnimationSequence,
    center_x: f32,
    center_y: f32,
    smoothness: f32,
    aspect_ratio: f32,
    width: usize,
    height: usize,
}

impl Disc {
    pub(crate) fn new(duration: f32, center_x: f32, center_y: f32, smoothness: f32, dimensions: (u32, u32)) -> Self {
        let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;
        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), center_x, center_y, smoothness, aspect_ratio, width: dimensions.0 as usize, height: dimensions.1 as usize }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let mapped_smoothness = 0.001 + 0.499 * self.smoothness * self.smoothness;

        let center = (self.center_x * self.aspect_ratio, self.center_y);

        let mut max_dist = 0.0_f32;
        for x in [0.0, self.aspect_ratio] {
            for y in [0.0, 1.0] {
                max_dist = max_dist.max(distance(center, (x, y)));
            }
        }

        let radius = progress * (max_dist + 2.0 * mapped_smoothness) - mapped_smoothness;
        let row_stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 / self.width as f32;
                let dist = distance((uv_x * self.aspect_ratio, uv_y), center);
                let factor = smoothstep(radius - mapped_smoothness, radius + mapped_smoothness, dist);

                for (c, &t) in canvas_px.iter_mut().zip(target_px) {
                    *c = (t as f32 * (1.0 - factor) + *c as f32 * factor + 0.5) as u8;
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
