use anyhow::{Context, Result};
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::RgbaImage;

pub(super) struct Scale {
    resizer: Resizer,
    filter: FilterType,
}

impl Scale {
    pub(super) fn new(filter: FilterType) -> Self {
        Self { resizer: Resizer::new(), filter }
    }

    pub(super) fn scale_exact(&mut self, src: &RgbaImage, width: u32, height: u32) -> Result<RgbaImage> {
        let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(self.filter));
        self.run(src, width, height, &options)
    }

    pub(super) fn scale_to_fill(&mut self, src: &RgbaImage, width: u32, height: u32, centering: (f64, f64)) -> Result<RgbaImage> {
        let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(self.filter)).fit_into_destination(Some(centering));
        self.run(src, width, height, &options)
    }

    fn run(&mut self, src: &RgbaImage, width: u32, height: u32, options: &ResizeOptions) -> Result<RgbaImage> {
        let (src_w, src_h) = (src.width(), src.height());
        let src_ref = ImageRef::new(src_w, src_h, src.as_raw(), PixelType::U8x4).context("failed to create image reference")?;

        let mut dst = Image::new(width, height, PixelType::U8x4);
        self.resizer.resize(&src_ref, &mut dst, Some(options)).context("resize operation failed")?;

        RgbaImage::from_raw(width, height, dst.into_vec()).context("failed to construct RGBA image after resize")
    }
}
