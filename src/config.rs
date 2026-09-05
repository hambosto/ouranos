use std::path::PathBuf;

use anyhow::{Context, Result};
use fast_image_resize::FilterType;
use figment::Figment;
use figment::providers::{Format, Toml};
use hex_color::HexColor;
use serde::Deserialize;
use xdg::BaseDirectories;

const CONFIG_PREFIX: &str = "ouranos";
const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) image: ImageConfig,
    #[serde(default)]
    pub(crate) transition: TransitionConfig,
    #[serde(default)]
    pub(crate) resize: ResizeConfig,
}

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let path = BaseDirectories::with_prefix(CONFIG_PREFIX).find_config_file(CONFIG_FILE).context("failed to find config file")?;
        tracing::info!(path = %path.display(), "reading configuration");

        Figment::from(Toml::file(&path)).extract().context("cannot parse config")
    }
}

#[derive(Deserialize)]
pub(crate) struct ImageConfig {
    pub(crate) path: PathBuf,
}

#[derive(Deserialize)]
pub(crate) struct TransitionConfig {
    pub(crate) transition_type: TransitionType,
    pub(crate) duration: f32,
    pub(crate) edge_smoothness: f32,
    pub(crate) transition_color: HexColor,
    pub(crate) wipe: WipeConfig,
    pub(crate) disc: DiscConfig,
    pub(crate) stripes: StripesConfig,
    pub(crate) honeycomb: HoneycombConfig,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            transition_type: TransitionType::Fade,
            duration: 1.5,
            edge_smoothness: 0.3,
            transition_color: HexColor::BLACK,
            wipe: WipeConfig::default(),
            disc: DiscConfig::default(),
            stripes: StripesConfig::default(),
            honeycomb: HoneycombConfig::default(),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct WipeConfig {
    pub(crate) direction: f32,
}

impl Default for WipeConfig {
    fn default() -> Self {
        Self { direction: 0.0 }
    }
}

#[derive(Deserialize)]
pub(crate) struct DiscConfig {
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
}

impl Default for DiscConfig {
    fn default() -> Self {
        Self { center_x: 0.5, center_y: 0.5 }
    }
}

#[derive(Deserialize)]
pub(crate) struct StripesConfig {
    pub(crate) stripe_count: f32,
    pub(crate) angle: f32,
}

impl Default for StripesConfig {
    fn default() -> Self {
        Self { stripe_count: 12.0, angle: 30.0 }
    }
}

#[derive(Deserialize)]
pub(crate) struct HoneycombConfig {
    pub(crate) cell_size: f32,
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
}

impl Default for HoneycombConfig {
    fn default() -> Self {
        Self { cell_size: 0.04, center_x: 0.5, center_y: 0.5 }
    }
}

#[derive(Deserialize)]
pub(crate) struct ResizeConfig {
    pub(crate) strategy: ResizeStrategy,
    pub(crate) crop_gravity: CropGravity,
    pub(crate) fill_color: [u8; 4],
    pub(crate) filter: Filter,
}

impl Default for ResizeConfig {
    fn default() -> Self {
        Self { strategy: ResizeStrategy::Crop, crop_gravity: CropGravity::Center, fill_color: [0, 0, 0, 255], filter: Filter::Lanczos3 }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResizeStrategy {
    No,
    Crop,
    Fit,
    Stretch,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CropGravity {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl CropGravity {
    pub(crate) fn as_centering(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
        }
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Filter {
    Nearest,
    Bilinear,
    CatmullRom,
    Mitchell,
    Lanczos3,
}

impl From<Filter> for FilterType {
    fn from(f: Filter) -> Self {
        match f {
            Filter::Nearest => Self::Box,
            Filter::Bilinear => Self::Bilinear,
            Filter::CatmullRom => Self::CatmullRom,
            Filter::Mitchell => Self::Mitchell,
            Filter::Lanczos3 => Self::Lanczos3,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TransitionType {
    None,
    Fade,
    Wipe,
    Disc,
    Stripes,
    Zoom,
    Honeycomb,
}
