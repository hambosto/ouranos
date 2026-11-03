use image::RgbaImage;

pub(super) fn to_shm_bytes(mut image: RgbaImage) -> Vec<u8> {
    for px in image.pixels_mut() {
        px.0.swap(0, 2);
    }

    image.into_raw()
}

pub(super) fn copy_to_shm_bytes(image: &RgbaImage) -> Vec<u8> {
    let mut result = Vec::with_capacity(image.as_raw().len());
    for px in image.pixels() {
        result.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
    }

    result
}
