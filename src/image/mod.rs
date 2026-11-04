use std::path::Path;

use anyhow::Result;
use fast_image_resize::FilterType;
use image::RgbaImage;

use super::image::compositor::{blank, centered_crop, place_onto};
use super::image::geometry::{center_offset, fit_dimensions};
use super::image::pixel::to_shm_bytes;
use super::image::scale::Scale;
use super::image::source::Source;
use crate::config::{CropGravity, ResizeConfig, ResizeStrategy};

mod compositor;
mod geometry;
mod pixel;
mod scale;
mod source;

pub(crate) struct Image {
    source: Source,
}

impl Image {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let source = Source::load(path)?;
        let (width, height) = source.dimensions();
        tracing::info!(width, height, pixels = u64::from(width) * u64::from(height), "wallpaper image decoded");

        Ok(Self { source })
    }

    pub(crate) fn render(&self, width: u32, height: u32, resize: &ResizeConfig) -> Result<Vec<u8>> {
        if width == 0 || height == 0 {
            anyhow::bail!("render target dimensions must be non-zero");
        }

        if self.source.dimensions() == (width, height) {
            // Source must be kept for other outputs, so this one copy is the floor.
            return Ok(to_shm_bytes(self.source.as_rgba().clone()));
        }

        let rendered = match resize.strategy {
            ResizeStrategy::No => Self::render_no(&self.source, width, height, resize.fill_color),
            ResizeStrategy::Crop => Self::render_crop(&self.source, width, height, resize.crop_gravity, resize.filter.into())?,
            ResizeStrategy::Fit => Self::render_fit(&self.source, width, height, resize.fill_color, resize.filter.into())?,
            ResizeStrategy::Stretch => Self::render_stretch(&self.source, width, height, resize.filter.into())?,
        };

        Ok(to_shm_bytes(rendered))
    }

    fn render_no(source: &Source, width: u32, height: u32, fill_color: [u8; 4]) -> RgbaImage {
        let cropped = centered_crop(source.as_rgba(), width, height);
        let (offset_x, offset_y) = center_offset(width, height, cropped.width(), cropped.height());

        let mut canvas = blank(width, height, fill_color);
        place_onto(&mut canvas, &cropped, offset_x, offset_y);

        canvas
    }

    fn render_crop(source: &Source, width: u32, height: u32, gravity: CropGravity, filter: FilterType) -> Result<RgbaImage> {
        let mut scaler = Scale::new(filter);
        scaler.scale_to_fill(source.as_rgba(), width, height, gravity.as_centering())
    }

    fn render_fit(source: &Source, width: u32, height: u32, fill_color: [u8; 4], filter: FilterType) -> Result<RgbaImage> {
        let (src_w, src_h) = source.dimensions();
        if src_w == width || src_h == height {
            return Ok(Self::render_no(source, width, height, fill_color));
        }

        let (scaled_w, scaled_h) = fit_dimensions(src_w, src_h, width, height);

        let mut scale = Scale::new(filter);
        let scaled = scale.scale_exact(source.as_rgba(), scaled_w, scaled_h)?;

        let (offset_x, offset_y) = center_offset(width, height, scaled_w, scaled_h);
        let mut canvas = blank(width, height, fill_color);
        place_onto(&mut canvas, &scaled, offset_x, offset_y);

        Ok(canvas)
    }

    fn render_stretch(source: &Source, width: u32, height: u32, filter: FilterType) -> Result<RgbaImage> {
        let mut scale = Scale::new(filter);
        scale.scale_exact(source.as_rgba(), width, height)
    }
}
