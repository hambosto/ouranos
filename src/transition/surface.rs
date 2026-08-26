pub(crate) struct Surface {
    width: usize,
    height: usize,
    aspect_ratio: f32,
}

impl Surface {
    #[must_use]
    pub(crate) fn new((width, height): (u32, u32)) -> Self {
        Self { width: width as usize, height: height as usize, aspect_ratio: width as f32 / height as f32 }
    }

    #[must_use]
    pub(crate) fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub(crate) fn height(&self) -> usize {
        self.height
    }

    #[must_use]
    pub(crate) fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    #[must_use]
    pub(crate) fn stride(&self) -> usize {
        self.width * 4
    }

    pub(crate) fn for_each_pixel(&self, canvas: &mut [u8], target: &[u8], mut paint: impl FnMut(f32, f32, &mut [u8], &[u8])) {
        let stride = self.width * 4;

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(stride).zip(target.chunks_exact(stride)).enumerate() {
            let v = row as f32 / self.height as f32;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                paint(col as f32 / self.width as f32, v, canvas_px, target_px);
            }
        }
    }

    pub(crate) fn blend(&self, canvas: &mut [u8], target: &[u8], keep: impl Fn(f32, f32) -> f32) {
        self.for_each_pixel(canvas, target, |u, v, old_px, new_px| mix_pixel(old_px, new_px, keep(u, v)));
    }
}

#[inline]
fn mix_pixel(old_px: &mut [u8], new_px: &[u8], keep: f32) {
    let show = 1.0 - keep;
    for (o, &n) in old_px.iter_mut().zip(new_px) {
        *o = (n as f32 * show + *o as f32 * keep + 0.5) as u8;
    }
}

#[must_use]
#[inline]
pub(crate) fn sample_bilinear(buf: &[u8], stride: usize, x: f32, y: f32) -> [f32; 4] {
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let (gx, gy) = (1.0 - fx, 1.0 - fy);
    let base = y0 * stride + x0 * 4;

    let top = &buf[base..base + 8];
    let bottom = &buf[base + stride..base + stride + 8];

    let mut pixel = [0.0_f32; 4];
    for (k, channel) in pixel.iter_mut().enumerate() {
        let top_mix = top[k] as f32 * gx + top[k + 4] as f32 * fx;
        let bottom_mix = bottom[k] as f32 * gx + bottom[k + 4] as f32 * fx;
        *channel = top_mix * gy + bottom_mix * fy;
    }
    pixel
}
