use image::RgbaImage;

pub(super) fn to_shm_bytes(mut image: RgbaImage) -> Vec<u8> {
    for px in image.pixels_mut() {
        px.0.swap(0, 2);
        px.0[3] = 255;
    }

    image.into_raw()
}
