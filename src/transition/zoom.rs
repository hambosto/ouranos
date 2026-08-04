use super::animation::AnimationSequence;

pub(crate) struct Zoom {
    seq: AnimationSequence,
    width: usize,
    height: usize,
    stride: usize,
}

impl Zoom {
    pub(crate) fn new(duration: f32, (w, h): (u32, u32)) -> Self {
        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), width: w as usize, height: h as usize, stride: w as usize * 4 }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let inv_w = 1.0 / self.width as f32;
        let inv_h = 1.0 / self.height as f32;
        let w_max = (self.width - 1) as f32;
        let h_max = (self.height - 1) as f32;
        let zoom = 0.15;
        let s1 = 1.0 + zoom * progress;
        let s2 = 1.0 + zoom * (1.0 - progress);
        let inv_s1 = 1.0 / s1;
        let inv_s2 = 1.0 / s2;
        let a = 1.0 - progress;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(self.stride).zip(target.chunks_exact(self.stride)).enumerate() {
            let uv_y = row as f32 * inv_h;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 * inv_w;

                let x = ((uv_x - 0.5) * inv_s1 + 0.5) * w_max;
                let y = ((uv_y - 0.5) * inv_s1 + 0.5) * h_max;
                let x0 = x.floor() as usize;
                let y0 = y.floor() as usize;
                let fx = x - x0 as f32;
                let fy = y - y0 as f32;
                let f0 = 1.0 - fx;
                let f1 = 1.0 - fy;
                let i = y0 * self.stride + x0 * 4;

                let s1 = [
                    (target[i] as f32 * f0 + target[i + 4] as f32 * fx) * f1 + (target[i + self.stride] as f32 * f0 + target[i + self.stride + 4] as f32 * fx) * fy,
                    (target[i + 1] as f32 * f0 + target[i + 5] as f32 * fx) * f1 + (target[i + self.stride + 1] as f32 * f0 + target[i + self.stride + 5] as f32 * fx) * fy,
                    (target[i + 2] as f32 * f0 + target[i + 6] as f32 * fx) * f1 + (target[i + self.stride + 2] as f32 * f0 + target[i + self.stride + 6] as f32 * fx) * fy,
                    (target[i + 3] as f32 * f0 + target[i + 7] as f32 * fx) * f1 + (target[i + self.stride + 3] as f32 * f0 + target[i + self.stride + 7] as f32 * fx) * fy,
                ];

                let x = ((uv_x - 0.5) * inv_s2 + 0.5) * w_max;
                let y = ((uv_y - 0.5) * inv_s2 + 0.5) * h_max;
                let x0 = x.floor() as usize;
                let y0 = y.floor() as usize;
                let fx = x - x0 as f32;
                let fy = y - y0 as f32;
                let f0 = 1.0 - fx;
                let f1 = 1.0 - fy;
                let i = y0 * self.stride + x0 * 4;

                let s2 = [
                    (target[i] as f32 * f0 + target[i + 4] as f32 * fx) * f1 + (target[i + self.stride] as f32 * f0 + target[i + self.stride + 4] as f32 * fx) * fy,
                    (target[i + 1] as f32 * f0 + target[i + 5] as f32 * fx) * f1 + (target[i + self.stride + 1] as f32 * f0 + target[i + self.stride + 5] as f32 * fx) * fy,
                    (target[i + 2] as f32 * f0 + target[i + 6] as f32 * fx) * f1 + (target[i + self.stride + 2] as f32 * f0 + target[i + self.stride + 6] as f32 * fx) * fy,
                    (target[i + 3] as f32 * f0 + target[i + 7] as f32 * fx) * f1 + (target[i + self.stride + 3] as f32 * f0 + target[i + self.stride + 7] as f32 * fx) * fy,
                ];

                for (i, (c, _)) in canvas_px.iter_mut().zip(target_px).enumerate() {
                    let old = *c as f32;
                    let new = s1[i] * a + s2[i] * progress;
                    *c = (old * a + new * progress + 0.5) as u8;
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}
