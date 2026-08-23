use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;

use super::surface::Surface;
use crate::config::Config;
use crate::image::Image;

pub(super) struct State {
    pub(super) registry_state: RegistryState,
    pub(super) output_state: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    pub(super) shm: Shm,
    surfaces: Vec<Surface>,
    config: Option<Config>,
}

impl State {
    pub(super) fn bind(global_list: &GlobalList, queue_handle: &QueueHandle<Self>) -> Result<Self> {
        let registry_state = RegistryState::new(global_list);
        let output_state = OutputState::new(global_list, queue_handle);
        let compositor = CompositorState::bind(global_list, queue_handle).context("wl_compositor not available")?;
        let layer_shell = LayerShell::bind(global_list, queue_handle).context("zwlr_layer_shell_v1 not available")?;
        let shm = Shm::bind(global_list, queue_handle).context("wl_shm not available")?;

        Ok(Self { registry_state, output_state, compositor, layer_shell, shm, surfaces: Vec::new(), config: None })
    }

    fn find_surface_mut(&mut self, wl: &WlSurface) -> Option<&mut Surface> {
        self.surfaces.iter_mut().find(|s| s.layer_surface.wl_surface() == wl)
    }

    pub(super) fn create_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        for handle in self.output_state.outputs() {
            let Some(info) = self.output_state.info(&handle) else {
                continue;
            };

            let name = info.name.as_deref().unwrap_or("unknown");
            let description = info.description.as_deref().unwrap_or("unknown");

            if self.surfaces.iter().any(|s| s.output_name == name) {
                continue;
            }

            let Some((width, height)) = info
                .logical_size
                .filter(|(w, h)| *w > 0 && *h > 0)
                .or_else(|| info.modes.iter().find(|m| m.current).map(|m| m.dimensions))
            else {
                tracing::warn!(name, "no valid dimensions, skipping");
                continue;
            };

            let wl_surface = self.compositor.create_surface(queue_handle);
            let layer_surface = self.layer_shell.create_layer_surface(queue_handle, wl_surface, Layer::Background, Some("wallpaper-rs"), Some(&handle));
            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            layer_surface.commit();

            tracing::info!(name, description, width, height, "monitor detected, creating wallpaper surface");
            self.surfaces.push(Surface::new(layer_surface, width.cast_unsigned(), height.cast_unsigned(), name.to_string()));
        }
    }

    pub(super) fn apply_wallpaper(&mut self, config: Config, queue_handle: &QueueHandle<Self>) -> Result<()> {
        if self.surfaces.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        self.config = Some(config);
        self.render_pending(queue_handle)
    }

    fn render_pending(&mut self, queue_handle: &QueueHandle<Self>) -> Result<()> {
        let Some(config) = &self.config else {
            return Ok(());
        };

        let pending_count = self.surfaces.iter().filter(|s| s.configured && !s.rendered).count();
        if pending_count == 0 {
            return Ok(());
        }

        tracing::info!(
            pending_count,
            image = %config.image.path.display(),
            strategy = ?config.resize.strategy,
            "loading and resizing wallpaper"
        );

        let image = Image::open(&config.image.path)?;
        let config = self.config.as_ref().context("config not set")?;
        for surface in &mut self.surfaces {
            if !surface.configured || surface.rendered {
                continue;
            }
            surface.begin_transition(&image, config, &self.shm)?;
            surface.commit(&self.shm, queue_handle)?;
        }

        tracing::info!(outputs = pending_count, "initial wallpaper rendered");

        Ok(())
    }

    pub(super) fn handle_configure(&mut self, wl_surface: &WlSurface, queue_handle: &QueueHandle<Self>) {
        let Some(surface) = self.find_surface_mut(wl_surface) else {
            return;
        };

        tracing::debug!(width = surface.width, height = surface.height, "compositor acknowledged surface, ready to render");
        surface.configure();

        if let Err(e) = self.render_pending(queue_handle) {
            tracing::warn!(?e, "failed to apply pending wallpaper");
        }
    }

    pub(super) fn handle_frame_callback(&mut self, wl_surface: &WlSurface, queue_handle: &QueueHandle<Self>) {
        let Some(idx) = self.surfaces.iter().position(|s| s.layer_surface.wl_surface() == wl_surface) else {
            return;
        };

        if self.surfaces[idx].transition.is_none() {
            return;
        }

        if let Err(e) = self.surfaces[idx].tick_transition(&self.shm, queue_handle) {
            tracing::warn!(?e, "failed to tick transition");
        }
    }

    pub(super) fn handle_output_destroyed(&mut self, wl_output: &WlOutput) {
        let Some(info) = self.output_state.info(wl_output) else {
            return;
        };

        let name = info.name.as_deref().unwrap_or("unknown");
        let mut removed = 0;

        self.surfaces.retain(|s| {
            if s.output_name != name {
                return true;
            }
            s.layer_surface.wl_surface().destroy();
            removed += 1;
            false
        });

        if removed > 0 {
            tracing::info!(name, removed, "wallpaper surfaces destroyed for disconnected output");
        }
    }
}
