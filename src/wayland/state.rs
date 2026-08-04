use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell};
use smithay_client_toolkit::shm::Shm;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
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

        tracing::info!(compositor = true, layer_shell = true, shm = true, "wayland globals bound");
        Ok(Self { registry_state, output_state, compositor, layer_shell, shm, surfaces: Vec::new(), config: None })
    }

    pub(super) fn create_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        for handle in self.output_state.outputs() {
            let Some(info) = self.output_state.info(&handle) else {
                continue;
            };

            let name = info.name.as_deref().unwrap_or("unknown");
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

            tracing::info!(name, width, height, "monitor detected, creating wallpaper surface");
            self.surfaces.push(Surface::new(layer_surface, width.cast_unsigned(), height.cast_unsigned()));
        }
    }

    pub(super) fn apply_wallpaper(&mut self, config: Config, queue_handle: &QueueHandle<Self>) -> Result<()> {
        if self.surfaces.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        for surface in &mut self.surfaces {
            surface.transition = None;
            surface.rendered = false;
        }

        self.config = Some(config);
        self.render_pending(queue_handle)
    }

    fn render_pending(&mut self, queue_handle: &QueueHandle<Self>) -> Result<()> {
        let Some(config) = &self.config else {
            return Ok(());
        };

        let pending: Vec<_> = self.surfaces.iter().filter(|s| s.configured && !s.rendered).collect();
        if pending.is_empty() {
            return Ok(());
        }

        let pending_count = pending.len();
        tracing::info!(pending_count, image = %config.image.path.display(), strategy = ?config.resize.strategy, "loading and resizing wallpaper");
        let image = Image::open(&config.image.path)?;

        for surface in self.surfaces.iter_mut().filter(|s| s.configured && !s.rendered) {
            surface.begin_transition(&image, config)?;
            surface.rendered = true;
            surface.commit(&self.shm, queue_handle)?;
        }
        tracing::info!(outputs = pending_count, "initial wallpaper rendered");

        Ok(())
    }

    pub(super) fn handle_configure(&mut self, wl_surface: &WlSurface, queue_handle: &QueueHandle<Self>) {
        if let Some(surface) = self.surfaces.iter_mut().find(|s| s.layer_surface.wl_surface() == wl_surface) {
            tracing::info!(width = surface.width, height = surface.height, "compositor acknowledged surface, ready to render");
            surface.configured = true;
        }

        if let Err(e) = self.render_pending(queue_handle) {
            tracing::warn!(?e, "failed to apply pending wallpaper");
        }
    }

    pub(super) fn handle_frame_callback(&mut self, wl_surface: &WlSurface, queue_handle: &QueueHandle<Self>) {
        let Some(surface) = self.surfaces.iter_mut().find(|s| s.layer_surface.wl_surface() == wl_surface) else {
            return;
        };

        if surface.transition.is_none() {
            return;
        }

        surface.tick();
        if let Err(e) = surface.commit(&self.shm, queue_handle) {
            tracing::warn!(?e, "failed to commit frame");
        }
    }
}
