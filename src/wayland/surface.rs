use smithay_client_toolkit::compositor::{CompositorState, Region};
use smithay_client_toolkit::output::OutputInfo;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;

use super::frame::SurfaceFrame;
use super::state::State;
use crate::config::Config;
use crate::image::Image;

pub(super) struct Unconfigured;
pub(super) struct Pending;
pub(super) struct Ready;
pub(super) struct Animating {
    frame: SurfaceFrame,
}

struct SurfaceData {
    layer: LayerSurface,
    output: WlOutput,
    logical_width: u32,
    logical_height: u32,
    scale_factor: u32,
}

pub(super) struct Surface<Stage> {
    data: SurfaceData,
    stage: Stage,
}

pub(super) enum ManagedSurface {
    Unconfigured(Surface<Unconfigured>),
    Pending(Surface<Pending>),
    Animating(Surface<Animating>),
    Ready(Surface<Ready>),
}

impl Surface<Unconfigured> {
    pub(super) fn new(compositor: &CompositorState, layer_shell: &LayerShell, output: WlOutput, info: &OutputInfo, queue_handle: &QueueHandle<State>) -> Option<Self> {
        let name = info.name.as_deref().unwrap_or("unknown");
        let description = info.description.as_deref().unwrap_or("unknown");

        let Some((logical_width, logical_height)) = info
            .logical_size
            .filter(|(width, height)| *width > 0 && *height > 0)
            .map(|(width, height)| (width.cast_unsigned(), height.cast_unsigned()))
            .or_else(|| {
                let fallback_scale = info.scale_factor.max(1);
                info.modes
                    .iter()
                    .find(|mode| mode.current)
                    .map(|mode| ((mode.dimensions.0 / fallback_scale).max(1).cast_unsigned(), (mode.dimensions.1 / fallback_scale).max(1).cast_unsigned()))
            })
        else {
            tracing::warn!(name, "no valid dimensions, skipping output");
            return None;
        };

        let requested_scale: u32 = info.scale_factor.max(1).cast_unsigned();
        let wl_surface = compositor.create_surface(queue_handle);
        let layer = layer_shell.create_layer_surface(queue_handle, wl_surface, Layer::Background, Some(env!("CARGO_PKG_NAME")), Some(&output));

        layer.set_anchor(Anchor::all());
        layer.set_exclusive_zone(-1);
        layer.set_size(0, 0);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);

        if let Ok(region) = Region::new(compositor) {
            layer.set_input_region(Some(region.wl_region()));
        } else {
            tracing::warn!(name, "failed to create input region, surface may take input");
        }

        let applied_scale = layer.set_buffer_scale(requested_scale).map_or(1, |()| requested_scale);
        layer.commit();

        tracing::info!(name, description, logical_width, logical_height, applied_scale, "monitor detected, creating wallpaper surface");

        Some(Self { data: SurfaceData { layer, output, logical_width, logical_height, scale_factor: applied_scale }, stage: Unconfigured })
    }

    pub(super) fn configure(self, logical_size: (u32, u32)) -> ManagedSurface {
        let mut data = self.data;
        let (logical_width, logical_height) = logical_size;
        if logical_width > 0 && logical_height > 0 && (logical_width != data.logical_width || logical_height != data.logical_height) {
            tracing::info!(old_width = data.logical_width, old_height = data.logical_height, logical_width, logical_height, "compositor requested new surface size, resizing");
            (data.logical_width, data.logical_height) = (logical_width, logical_height);
        }
        ManagedSurface::Pending(Surface { data, stage: Pending })
    }
}

impl Surface<Pending> {
    pub(super) fn start_transition(self, image: &Image, config: &Config, shm: &Shm, queue_handle: &QueueHandle<State>) -> ManagedSurface {
        let Some(buffer_width) = self.data.logical_width.checked_mul(self.data.scale_factor) else {
            tracing::warn!("surface dimensions too large");
            return ManagedSurface::Pending(self);
        };
        let Some(buffer_height) = self.data.logical_height.checked_mul(self.data.scale_factor) else {
            tracing::warn!("surface dimensions too large");
            return ManagedSurface::Pending(self);
        };
        let frame = match SurfaceFrame::new(image, config, shm, buffer_width, buffer_height) {
            Ok(frame) => frame,
            Err(e) => {
                tracing::warn!(?e, "failed to start transition for output");
                return ManagedSurface::Pending(self);
            }
        };
        Surface { data: self.data, stage: Animating { frame } }.advance_frame(queue_handle)
    }
}

impl Surface<Animating> {
    pub(super) fn advance_frame(self, queue_handle: &QueueHandle<State>) -> ManagedSurface {
        let Surface { data, stage } = self;
        let mut frame = stage.frame;
        match frame.commit(&data.layer, queue_handle) {
            Ok(true) => ManagedSurface::Ready(Surface { data, stage: Ready }),
            Ok(false) => ManagedSurface::Animating(Surface { data, stage: Animating { frame } }),
            Err(e) => {
                tracing::warn!(?e, "failed to tick transition, will retry on next render");
                ManagedSurface::Pending(Surface { data, stage: Pending })
            }
        }
    }
}

impl Surface<Ready> {
    pub(super) fn configure(self, logical_size: (u32, u32)) -> ManagedSurface {
        let mut data = self.data;
        let (logical_width, logical_height) = logical_size;
        if logical_width > 0 && logical_height > 0 && (logical_width != data.logical_width || logical_height != data.logical_height) {
            tracing::info!(old_width = data.logical_width, old_height = data.logical_height, logical_width, logical_height, "compositor requested new surface size, resizing");
            (data.logical_width, data.logical_height) = (logical_width, logical_height);
            ManagedSurface::Pending(Surface { data, stage: Pending })
        } else {
            ManagedSurface::Ready(Surface { data, stage: Ready })
        }
    }
}

impl ManagedSurface {
    pub(super) fn is_wl_surface(&self, surface: &WlSurface) -> bool {
        self.wl_surface() == surface
    }

    pub(super) fn is_layer_surface(&self, layer: &LayerSurface) -> bool {
        self.wl_surface() == layer.wl_surface()
    }

    pub(super) fn is_output(&self, output: &WlOutput) -> bool {
        self.output() == output
    }

    fn wl_surface(&self) -> &WlSurface {
        match self {
            Self::Unconfigured(surface) => surface.data.layer.wl_surface(),
            Self::Pending(surface) => surface.data.layer.wl_surface(),
            Self::Animating(surface) => surface.data.layer.wl_surface(),
            Self::Ready(surface) => surface.data.layer.wl_surface(),
        }
    }

    pub(super) fn output(&self) -> &WlOutput {
        match self {
            Self::Unconfigured(surface) => &surface.data.output,
            Self::Pending(surface) => &surface.data.output,
            Self::Animating(surface) => &surface.data.output,
            Self::Ready(surface) => &surface.data.output,
        }
    }

    pub(super) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    pub(super) fn configure(self, logical_size: (u32, u32)) -> Self {
        match self {
            Self::Unconfigured(surface) => surface.configure(logical_size),
            Self::Pending(mut surface) => {
                let (logical_width, logical_height) = logical_size;
                if logical_width > 0 && logical_height > 0 && (logical_width != surface.data.logical_width || logical_height != surface.data.logical_height) {
                    tracing::info!(old_width = surface.data.logical_width, old_height = surface.data.logical_height, logical_width, logical_height, "compositor requested new surface size, resizing");
                    (surface.data.logical_width, surface.data.logical_height) = (logical_width, logical_height);
                }
                Self::Pending(surface)
            }
            Self::Animating(surface) => {
                let (logical_width, logical_height) = logical_size;
                if logical_width > 0 && logical_height > 0 && (logical_width != surface.data.logical_width || logical_height != surface.data.logical_height) {
                    let Surface { mut data, stage } = surface;
                    drop(stage.frame);
                    tracing::info!(old_width = data.logical_width, old_height = data.logical_height, logical_width, logical_height, "compositor requested new surface size, resizing");
                    (data.logical_width, data.logical_height) = (logical_width, logical_height);
                    Self::Pending(Surface { data, stage: Pending })
                } else {
                    Self::Animating(surface)
                }
            }
            Self::Ready(surface) => surface.configure(logical_size),
        }
    }

    pub(super) fn rescale(self, scale_factor: i32) -> Self {
        let scale_factor: u32 = scale_factor.max(1).cast_unsigned();
        match self {
            Self::Unconfigured(mut surface) => {
                if scale_factor == surface.data.scale_factor {
                    return Self::Unconfigured(surface);
                }
                if let Err(e) = surface.data.layer.set_buffer_scale(scale_factor) {
                    tracing::warn!(?e, "compositor does not support buffer scaling, keeping current resolution");
                    return Self::Unconfigured(surface);
                }
                tracing::info!(old_scale = surface.data.scale_factor, scale_factor, "output scale changed, resizing");
                surface.data.scale_factor = scale_factor;
                Self::Pending(Surface { data: surface.data, stage: Pending })
            }
            Self::Pending(mut surface) => {
                if scale_factor == surface.data.scale_factor {
                    return Self::Pending(surface);
                }
                if let Err(e) = surface.data.layer.set_buffer_scale(scale_factor) {
                    tracing::warn!(?e, "compositor does not support buffer scaling, keeping current resolution");
                    return Self::Pending(surface);
                }
                tracing::info!(old_scale = surface.data.scale_factor, scale_factor, "output scale changed, resizing");
                surface.data.scale_factor = scale_factor;
                Self::Pending(surface)
            }
            Self::Animating(surface) => {
                if scale_factor == surface.data.scale_factor {
                    return Self::Animating(surface);
                }
                let Surface { mut data, stage } = surface;
                if let Err(e) = data.layer.set_buffer_scale(scale_factor) {
                    tracing::warn!(?e, "compositor does not support buffer scaling, keeping current resolution");
                    return Self::Animating(Surface { data, stage: Animating { frame: stage.frame } });
                }
                drop(stage.frame);
                tracing::info!(old_scale = data.scale_factor, scale_factor, "output scale changed, resizing");
                data.scale_factor = scale_factor;
                Self::Pending(Surface { data, stage: Pending })
            }
            Self::Ready(surface) => {
                let mut data = surface.data;
                if scale_factor == data.scale_factor {
                    return Self::Ready(Surface { data, stage: Ready });
                }
                if let Err(e) = data.layer.set_buffer_scale(scale_factor) {
                    tracing::warn!(?e, "compositor does not support buffer scaling, keeping current resolution");
                    return Self::Ready(Surface { data, stage: Ready });
                }
                tracing::info!(old_scale = data.scale_factor, scale_factor, "output scale changed, resizing");
                data.scale_factor = scale_factor;
                Self::Pending(Surface { data, stage: Pending })
            }
        }
    }

    pub(super) fn start_transition(self, image: &Image, config: &Config, shm: &Shm, queue_handle: &QueueHandle<State>) -> Self {
        match self {
            Self::Pending(surface) => surface.start_transition(image, config, shm, queue_handle),
            other => other,
        }
    }

    pub(super) fn advance_frame(self, queue_handle: &QueueHandle<State>) -> Self {
        match self {
            Self::Animating(surface) => surface.advance_frame(queue_handle),
            other => other,
        }
    }
}
