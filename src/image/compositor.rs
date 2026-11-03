use image::{GenericImageView, Rgba, RgbaImage};

use crate::image::geometry::crop_origin;

pub(super) fn blank(width: u32, height: u32, fill_color: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(width, height, Rgba(fill_color))
}

pub(super) fn place_onto(canvas: &mut RgbaImage, src: &RgbaImage, x: i64, y: i64) {
    image::imageops::overlay(canvas, src, x, y);
}

pub(super) fn centered_crop(src: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let (src_w, src_h) = (src.width(), src.height());

    if src_w > width || src_h > height {
        let (x, y) = crop_origin(src_w, src_h, width, height);
        src.view(x, y, width.min(src_w), height.min(src_h)).to_image()
    } else {
        src.view(0, 0, src_w, src_h).to_image()
    }
}
