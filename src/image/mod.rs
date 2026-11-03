use std::path::Path;

use anyhow::Result;
use fast_image_resize::FilterType;
use image::RgbaImage;

use super::image::compositor::{blank, centered_crop, place_onto};
use super::image::geometry::{center_offset, fit_dimensions};
use super::image::pixel::{copy_to_shm_bytes, to_shm_bytes};
use super::image::scale::Scale;
use super::image::source::Source;
use super::image::target::Target;
use crate::config::{CropGravity, ResizeConfig};

mod compositor;
mod geometry;
mod pixel;
mod scale;
mod source;
mod target;

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
        let target = Target::new(width, height)?;

        if self.source.dimensions() == target.dimensions() {
            return Ok(copy_to_shm_bytes(self.source.as_rgba()));
        }

        let rendered = match resize.strategy {
            crate::config::ResizeStrategy::No => Self::render_no(&self.source, &target, resize.fill_color),
            crate::config::ResizeStrategy::Crop => Self::render_crop(&self.source, &target, resize.crop_gravity, resize.filter.into())?,
            crate::config::ResizeStrategy::Fit => Self::render_fit(&self.source, &target, resize.fill_color, resize.filter.into())?,
            crate::config::ResizeStrategy::Stretch => Self::render_stretch(&self.source, &target, resize.filter.into())?,
        };

        Ok(to_shm_bytes(rendered))
    }

    fn render_no(source: &Source, target: &Target, fill_color: [u8; 4]) -> RgbaImage {
        let cropped = centered_crop(source.as_rgba(), target.width(), target.height());
        let (offset_x, offset_y) = center_offset(target.width(), target.height(), cropped.width(), cropped.height());

        let mut canvas = blank(target.width(), target.height(), fill_color);
        place_onto(&mut canvas, &cropped, offset_x, offset_y);

        canvas
    }

    fn render_crop(source: &Source, target: &Target, gravity: CropGravity, filter: FilterType) -> Result<RgbaImage> {
        let mut scaler = Scale::new(filter);
        scaler.scale_to_fill(source.as_rgba(), target.width(), target.height(), gravity.as_centering())
    }

    fn render_fit(source: &Source, target: &Target, fill_color: [u8; 4], filter: FilterType) -> Result<RgbaImage> {
        let (src_w, src_h) = source.dimensions();
        let (scaled_w, scaled_h) = fit_dimensions(src_w, src_h, target.width(), target.height());

        let mut scale = Scale::new(filter);
        let scaled = scale.scale_exact(source.as_rgba(), scaled_w, scaled_h)?;

        let (offset_x, offset_y) = center_offset(target.width(), target.height(), scaled_w, scaled_h);
        let mut canvas = blank(target.width(), target.height(), fill_color);
        place_onto(&mut canvas, &scaled, offset_x, offset_y);

        Ok(canvas)
    }

    fn render_stretch(source: &Source, target: &Target, filter: FilterType) -> Result<RgbaImage> {
        let mut scale = Scale::new(filter);
        scale.scale_exact(source.as_rgba(), target.width(), target.height())
    }
}
