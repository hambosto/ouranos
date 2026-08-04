use super::animation::AnimationSequence;

pub(crate) struct Wipe {
    seq: AnimationSequence,
    along_x: bool,
    reversed: bool,
    smoothness: f32,
    extended_max: f32,
    width: usize,
    height: usize,
}

impl Wipe {
    pub(crate) fn new(duration: f32, direction: f32, smoothness: f32, (w, h): (u32, u32)) -> Self {
        let dir = direction as u32;
        let s = 0.001 + 0.499 * smoothness * smoothness;

        Self {
            seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0),
            along_x: dir == 0 || dir == 1,
            reversed: dir == 0 || dir == 2,
            smoothness: s,
            extended_max: 1.0 + 2.0 * s,
            width: w as usize,
            height: h as usize,
        }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let s = self.smoothness;
        let extended = progress * self.extended_max - s;
        let edge = if self.reversed { 1.0 - extended } else { extended };
        let row_stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv = if self.along_x { col as f32 / self.width as f32 } else { row as f32 / self.height as f32 };

                let t = ((uv - edge + s) / (2.0 * s)).clamp(0.0, 1.0);
                let factor = t * t * (3.0 - 2.0 * t);

                let (f1, f2) = if self.reversed { (1.0 - factor, factor) } else { (factor, 1.0 - factor) };

                for (c, &t) in canvas_px.iter_mut().zip(target_px) {
                    *c = (*c as f32 * f1 + t as f32 * f2 + 0.5) as u8;
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}
