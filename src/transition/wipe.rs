use super::animation::AnimationSequence;

pub(crate) struct Wipe {
    seq: AnimationSequence,
    direction: f32,
    smoothness: f32,
    width: usize,
    height: usize,
}

impl Wipe {
    pub(crate) fn new(duration: f32, direction: f32, smoothness: f32, dimensions: (u32, u32)) -> Self {
        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), direction, smoothness, width: dimensions.0 as usize, height: dimensions.1 as usize }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let mapped_smoothness = 0.001 + 0.499 * self.smoothness * self.smoothness;
        let extended_progress = progress * (1.0 + 2.0 * mapped_smoothness) - mapped_smoothness;

        let direction = self.direction as u32;
        let along_x = direction == 0 || direction == 1;
        let reversed = direction == 0 || direction == 2;
        let row_stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 / self.width as f32;
                let uv = if along_x { uv_x } else { uv_y };

                let edge = if reversed { 1.0 - extended_progress } else { extended_progress };
                let factor = smoothstep(edge - mapped_smoothness, edge + mapped_smoothness, uv);
                let (f1, f2) = if reversed { (1.0 - factor, factor) } else { (factor, 1.0 - factor) };

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
