use super::animation::AnimationSequence;

pub(crate) struct Honeycomb {
    seq: AnimationSequence,
    size: f32,
    origin: (f32, f32),
    smoothness: f32,
    radius_max: f32,
    inv_size: f32,
    sqrt3: f32,
    width: usize,
    height: usize,
    aspect_ratio: f32,
}

impl Honeycomb {
    pub(crate) fn new(duration: f32, cell_size: f32, center_x: f32, center_y: f32, smoothness: f32, (w, h): (u32, u32)) -> Self {
        let aspect_ratio = w as f32 / h as f32;
        let origin = (center_x * aspect_ratio, center_y);
        let s = 0.001 + 0.499 * smoothness * smoothness;
        let sqrt3 = 3.0_f32.sqrt();

        let mut max_dist = 0.0_f32;
        for x in [0.0, aspect_ratio] {
            for y in [0.0, 1.0] {
                let dx = origin.0 - x;
                let dy = origin.1 - y;
                max_dist = max_dist.max((dx * dx + dy * dy).sqrt());
            }
        }

        Self {
            seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0),
            size: cell_size,
            origin,
            smoothness: s,
            radius_max: max_dist + 2.0 * s,
            inv_size: 1.0 / cell_size,
            sqrt3,
            width: w as usize,
            height: h as usize,
            aspect_ratio,
        }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let s = self.smoothness;
        let radius = progress * self.radius_max - s;
        let row_stride = self.width * 4;
        let sqrt3 = self.sqrt3;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 / self.width as f32;
                let ax = uv_x * self.aspect_ratio;

                let q = ax * (2.0 / 3.0) * self.inv_size;
                let r = (-ax / 3.0 + sqrt3 / 3.0 * uv_y) * self.inv_size;

                let y = -q - r;
                let mut rx = (q + 0.5).floor();
                let mut rz = (r + 0.5).floor();
                let ry = (y + 0.5).floor();

                let dx = (rx - q).abs();
                let dy = (ry - y).abs();
                let dz = (rz - r).abs();

                if dx > dy && dx > dz {
                    rx = -ry - rz;
                } else if dy <= dz {
                    rz = -rx - ry;
                }

                let cx = rx * 1.5 * self.size;
                let cy = (rx * sqrt3 / 2.0 + rz * sqrt3) * self.size;

                let ddx = self.origin.0 - cx;
                let ddy = self.origin.1 - cy;
                let dist = (ddx * ddx + ddy * ddy).sqrt();

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
