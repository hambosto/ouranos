pub(crate) struct Surface {
    width: usize,
    height: usize,
    aspect_ratio: f32,
}

impl Surface {
    #[must_use]
    pub(crate) fn new((width, height): (u32, u32)) -> Self {
        Self { width: width as usize, height: height as usize, aspect_ratio: width as f32 / height.max(1) as f32 }
    }

    #[must_use]
    pub(crate) fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    pub(crate) fn for_each_pixel(&self, canvas: &mut [u8], target: &[u8], mut paint: impl FnMut(f32, f32, &mut [u8], &[u8])) {
        debug_assert_eq!(canvas.len(), target.len());
        if self.width == 0 || self.height == 0 {
            return;
        }
        let stride = self.width * 4;
        let (inv_w, inv_h) = (1.0 / self.width as f32, 1.0 / self.height as f32);

        for (row, (canvas_row, target_row)) in canvas.chunks_exact_mut(stride).zip(target.chunks_exact(stride)).enumerate() {
            let v = row as f32 * inv_h;

            for (col, (canvas_px, target_px)) in canvas_row.chunks_exact_mut(4).zip(target_row.chunks_exact(4)).enumerate() {
                paint(col as f32 * inv_w, v, canvas_px, target_px);
            }
        }
    }

    pub(crate) fn blend(&self, canvas: &mut [u8], target: &[u8], keep: impl Fn(f32, f32) -> f32) {
        self.for_each_pixel(canvas, target, |u, v, old_px, new_px| mix_pixel(old_px, new_px, keep(u, v)));
    }

    pub(crate) fn blend_flat(&self, canvas: &mut [u8], target: &[u8], keep: f32) {
        debug_assert_eq!(canvas.len(), target.len());
        for (old_px, new_px) in canvas.chunks_exact_mut(4).zip(target.chunks_exact(4)) {
            mix_pixel(old_px, new_px, keep);
        }
    }

    #[must_use]
    pub(crate) fn sample(&self, buf: &[u8], x: f32, y: f32) -> [f32; 4] {
        debug_assert_eq!(buf.len(), self.width * 4 * self.height);
        let (w, h) = (self.width.max(1), self.height.max(1));
        let x = x.clamp(0.0, (w - 1) as f32);
        let y = y.clamp(0.0, (h - 1) as f32);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let (gx, gy) = (1.0 - fx, 1.0 - fy);

        let stride = w * 4;
        let mut pixel = [0.0_f32; 4];
        for (k, channel) in pixel.iter_mut().enumerate() {
            let top = buf[y0 * stride + x0 * 4 + k] as f32 * gx + buf[y0 * stride + x1 * 4 + k] as f32 * fx;
            let bottom = buf[y1 * stride + x0 * 4 + k] as f32 * gx + buf[y1 * stride + x1 * 4 + k] as f32 * fx;
            *channel = top * gy + bottom * fy;
        }
        pixel
    }
}

#[inline]
fn mix_pixel(old_px: &mut [u8], new_px: &[u8], keep: f32) {
    let show = 1.0 - keep;
    for (o, &n) in old_px.iter_mut().zip(new_px) {
        *o = (n as f32 * show + *o as f32 * keep + 0.5) as u8;
    }
}
