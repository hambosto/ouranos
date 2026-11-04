pub(super) fn fit_dimensions(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> (u32, u32) {
    let scale = (f64::from(dst_w) / f64::from(src_w)).min(f64::from(dst_h) / f64::from(src_h));
    let width = (f64::from(src_w) * scale).round() as u32;
    let height = (f64::from(src_h) * scale).round() as u32;

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
