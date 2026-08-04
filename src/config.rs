use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fast_image_resize::FilterType;
use hex_color::HexColor;
use serde::Deserialize;
use xdg::BaseDirectories;

const CONFIG_PREFIX: &str = "wallpaper-rs";
const CONFIG_FILE: &str = "config.toml";

impl Config {
    pub(crate) fn load() -> Result<Self> {
        let xdg_dirs = BaseDirectories::with_prefix(CONFIG_PREFIX);
        let config_file = xdg_dirs.find_config_file(CONFIG_FILE).context("failed to find config file")?;
        tracing::info!(path = %config_file.display(), prefix = CONFIG_PREFIX, "reading configuration file");

        Self::load_from_file(&config_file)
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("cannot read from config file")?;
        let config: Self = toml::from_str(&content).context("cannot parse config file")?;
        Ok(config)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) image: ImageConfig,
    #[serde(default)]
    pub(crate) transition: TransitionConfig,
    #[serde(default)]
    pub(crate) resize: ResizeConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImageConfig {
    pub(crate) path: PathBuf,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ResizeConfig {
    pub(crate) strategy: ResizeStrategy,
    pub(crate) crop_gravity: CropGravity,
    pub(crate) fill_color: [u8; 4],
    pub(crate) filter: Filter,
}

impl Default for ResizeConfig {
    fn default() -> Self {
        Self { strategy: ResizeStrategy::Crop, crop_gravity: CropGravity::Center, fill_color: [0x00, 0x00, 0x00, 0xFF], filter: Filter::Lanczos3 }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
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

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct WipeConfig {
    pub(crate) direction: f32,
}

impl Default for WipeConfig {
    fn default() -> Self {
        Self { direction: 0.0 }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DiscConfig {
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
}

impl Default for DiscConfig {
    fn default() -> Self {
        Self { center_x: 0.5, center_y: 0.5 }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct StripesConfig {
    pub(crate) stripe_count: f32,
    pub(crate) angle: f32,
}

impl Default for StripesConfig {
    fn default() -> Self {
        Self { stripe_count: 12.0, angle: 30.0 }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default, deny_unknown_fields)]
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

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResizeStrategy {
    No,
    Crop,
    Fit,
    Stretch,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TransitionType {
    None,
    Simple,
    Fade,
    Wipe,
    Disc,
    Stripes,
    Zoom,
    Honeycomb,
}
