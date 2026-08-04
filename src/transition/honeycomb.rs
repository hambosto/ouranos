use super::animation::AnimationSequence;

pub(crate) struct Honeycomb {
    seq: AnimationSequence,
    cell_size: f32,
    center_x: f32,
    center_y: f32,
    smoothness: f32,
    aspect_ratio: f32,
    width: usize,
    height: usize,
}

impl Honeycomb {
    pub(crate) fn new(duration: f32, cell_size: f32, center_x: f32, center_y: f32, smoothness: f32, dimensions: (u32, u32)) -> Self {
        let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;
        Self { seq: AnimationSequence::new(duration, 0.0, 1.0, 0.0), cell_size, center_x, center_y, smoothness, aspect_ratio, width: dimensions.0 as usize, height: dimensions.1 as usize }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let progress = self.seq.now();
        let mapped_smoothness = 0.001 + 0.499 * self.smoothness * self.smoothness;

        let size = self.cell_size;
        let wave_origin = (self.center_x * self.aspect_ratio, self.center_y);

        let mut max_dist = 0.0_f32;
        for x in [0.0, self.aspect_ratio] {
            for y in [0.0, 1.0] {
                max_dist = max_dist.max(distance(wave_origin, (x, y)));
            }
        }

        let radius = progress * (max_dist + 2.0 * mapped_smoothness) - mapped_smoothness;
        let row_stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(row_stride).zip(target.chunks_exact(row_stride)).enumerate() {
            let uv_y = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                let uv_x = col as f32 / self.width as f32;
                let aspect_uv = (uv_x * self.aspect_ratio, uv_y);

                let q = (aspect_uv.0 * (2.0 / 3.0)) / size;
                let r = ((-aspect_uv.0 / 3.0) + (3.0_f32.sqrt() / 3.0) * aspect_uv.1) / size;
                let (hex_q, hex_r) = hex_round(q, r);

                let cell_center = (hex_q * 1.5 * size, (hex_q * 3.0_f32.sqrt() / 2.0 + hex_r * 3.0_f32.sqrt()) * size);

                let dist = distance(cell_center, wave_origin);
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

fn hex_round(q: f32, r: f32) -> (f32, f32) {
    let round_half_up = |v: f32| (v + 0.5).floor();
    let y = -q - r;

    let mut rx = round_half_up(q);
    let mut rz = round_half_up(r);
    let ry = round_half_up(y);

    let dx = (rx - q).abs();
    let dy = (ry - y).abs();
    let dz = (rz - r).abs();

    if dx > dy && dx > dz {
        rx = -ry - rz;
    } else if dy <= dz {
        rz = -rx - ry;
    }

    (rx, rz)
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
