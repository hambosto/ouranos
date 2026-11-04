pub(super) fn fit_dimensions(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (u32, u32) {
    let ratio = dst_w as f32 / dst_h as f32;
    let img_r = src_w as f32 / src_h as f32;

    let (width, height) = if ratio > img_r {
        let scale = dst_h as f32 / src_h as f32;
        ((src_w as f32 * scale) as u32, dst_h)
    } else {
        let scale = dst_w as f32 / src_w as f32;
        (dst_w, (src_h as f32 * scale) as u32)
    };

    (width.max(1), height.max(1))
}

pub(super) fn center_offset(canvas_w: u32, canvas_h: u32, inner_w: u32, inner_h: u32) -> (i64, i64) {
    let x = i64::from(canvas_w.saturating_sub(inner_w) / 2);
    let y = i64::from(canvas_h.saturating_sub(inner_h) / 2);

    (x, y)
}

pub(super) fn crop_origin(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (u32, u32) {
    (src_w.saturating_sub(dst_w) / 2, src_h.saturating_sub(dst_h) / 2)
}
