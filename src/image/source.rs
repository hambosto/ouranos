use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};

pub(super) struct Source {
    rgba: RgbaImage,
}

impl Source {
    pub(super) fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).context("failed to open image")?;
        let reader = ImageReader::new(BufReader::new(file)).with_guessed_format().context("failed to detect image format")?;

        let mut decoder = reader.into_decoder().context("failed to create decoder")?;
        let orientation = decoder.orientation().context("failed to read orientation")?;

        let mut image = DynamicImage::from_decoder(decoder).context("failed to decode image")?;
        image.apply_orientation(orientation);

        Ok(Self { rgba: image.into_rgba8() })
    }

    pub(super) fn dimensions(&self) -> (u32, u32) {
        (self.rgba.width(), self.rgba.height())
    }

    pub(super) fn as_rgba(&self) -> &RgbaImage {
        &self.rgba
    }
}
