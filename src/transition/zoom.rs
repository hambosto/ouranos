use super::animation::AnimationSequence;

pub(crate) struct Zoom {
    seq: AnimationSequence,
    width: usize,
    height: usize,
}

impl Zoom {
    pub(crate) fn new(duration: f32, dimensions: (u32, u32)) -> Self {
        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), width: dimensions.0 as usize, height: dimensions.1 as usize }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let zoom = 0.15;

        let scale1 = 1.0 + zoom * progress;
        let scale2 = 1.0 + zoom * (1.0 - progress);
        let row_stride = self.width * 4;

        for (row, canvas_row) in canvas.chunks_exact_mut(row_stride).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, canvas_px) in canvas_row.chunks_exact_mut(4).enumerate() {
                let uv_x = col as f32 / self.width as f32;

                let uv1 = ((uv_x - 0.5) / scale1 + 0.5, (uv_y - 0.5) / scale1 + 0.5);
                let uv2 = ((uv_x - 0.5) / scale2 + 0.5, (uv_y - 0.5) / scale2 + 0.5);

                let sample1 = sample_bilinear(target, uv1.0, uv1.1, self.width, self.height);
                let sample2 = sample_bilinear(target, uv2.0, uv2.1, self.width, self.height);

                for (c, px) in canvas_px.iter_mut().enumerate() {
                    let old = *px as f32;
                    let new_val = sample1[c] as f32 * (1.0 - progress) + sample2[c] as f32 * progress;
                    *px = (old * (1.0 - progress) + new_val * progress + 0.5) as u8;
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}

fn sample_bilinear(data: &[u8], x: f32, y: f32, width: usize, height: usize) -> [u8; 4] {
    let x = (x * width as f32).clamp(0.0, (width - 1) as f32);
    let y = (y * height as f32).clamp(0.0, (height - 1) as f32);

    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let stride = width * 4;
    let i00 = y0 * stride + x0 * 4;
    let i10 = y0 * stride + x1 * 4;
    let i01 = y1 * stride + x0 * 4;
    let i11 = y1 * stride + x1 * 4;

    let mut result = [0u8; 4];
    for c in 0..4 {
        let v00 = data[i00 + c] as f32;
        let v10 = data[i10 + c] as f32;
        let v01 = data[i01 + c] as f32;
        let v11 = data[i11 + c] as f32;

        let top = v00 * (1.0 - fx) + v10 * fx;
        let bot = v01 * (1.0 - fx) + v11 * fx;
        result[c] = (top * (1.0 - fy) + bot * fy + 0.5) as u8;
    }
    result
}
