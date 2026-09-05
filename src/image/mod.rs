mod resize;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};

use crate::config::ResizeConfig;

pub(crate) struct Image {
    rgba: RgbaImage,
}

impl Image {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        if !path.is_file() {
            anyhow::bail!("path is not a file: {}", path.display());
        }

        let file = File::open(path).context("failed to open image")?;
        let reader = ImageReader::new(BufReader::new(file)).with_guessed_format().context("failed to detect image format")?;
        let mut decoder = reader.into_decoder().context("failed to create decoder")?;
        let orientation = decoder.orientation().context("failed to read orientation")?;
        let mut image = DynamicImage::from_decoder(decoder).context("failed to decode image")?;
        image.apply_orientation(orientation);

        let rgba = image.into_rgba8();
        tracing::info!(width = rgba.width(), height = rgba.height(), pixels = rgba.width() * rgba.height(), "wallpaper image decoded");

        Ok(Self { rgba })
    }

    pub(crate) fn render(&self, width: u32, height: u32, resize: &ResizeConfig) -> Result<Vec<u8>> {
        let mut resized = resize::apply(&self.rgba, width, height, resize)?;
        resized.pixels_mut().for_each(|px| px.0.swap(0, 2));

        Ok(resized.into_raw())
    }
}
