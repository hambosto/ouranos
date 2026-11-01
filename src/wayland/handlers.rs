use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use wayland_client::protocol::wl_output::{Transform, WlOutput};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, QueueHandle};

use super::state::State;
use super::surface::Status;

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    smithay_client_toolkit::registry_handlers!(OutputState);
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, queue_handle: &QueueHandle<Self>, _: WlOutput) {
        self.create_surfaces(queue_handle);
    }

    fn update_output(&mut self, _: &Connection, queue_handle: &QueueHandle<Self>, _: WlOutput) {
        self.create_surfaces(queue_handle);
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, wl_output: WlOutput) {
        let before = self.surfaces.len();
        self.surfaces.retain(|s| s.output != wl_output);
        if self.surfaces.len() != before {
            tracing::info!("wallpaper surface destroyed for disconnected output");
        }
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(&mut self, _: &Connection, queue_handle: &QueueHandle<Self>, wl_surface: &WlSurface, new_factor: i32) {
        let Some(surface) = self.find_surface_mut(wl_surface) else {
            return;
        };

        surface.rescale(new_factor);
        self.render(queue_handle);
    }

    fn transform_changed(&mut self, _: &Connection, queue_handle: &QueueHandle<Self>, wl_surface: &WlSurface, _: Transform) {
        let output = match self.find_surface_mut(wl_surface) {
            Some(surface) => surface.output.clone(),
            None => return,
        };
        let new_size = self
            .output_state
            .info(&output)
            .and_then(|info| info.logical_size)
            .filter(|(w, h)| *w > 0 && *h > 0)
            .map(|(w, h)| (w.cast_unsigned(), h.cast_unsigned()));

        let Some(surface) = self.find_surface_mut(wl_surface) else {
            return;
        };

        if let Some(size) = new_size {
            surface.configure(size);
        } else {
            surface.status = Status::Pending;
        }
        self.render(queue_handle);
    }

    fn frame(&mut self, _: &Connection, queue_handle: &QueueHandle<Self>, wl_surface: &WlSurface, _: u32) {
        let tick_error = match self.find_surface_mut(wl_surface) {
            Some(surface) => surface.tick(queue_handle).err(),
            None => return,
        };

        if let Some(e) = tick_error {
            tracing::warn!(?e, "failed to tick transition, will retry on next render");
            self.render(queue_handle);
        }
    }

    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _: &WlOutput) {}

    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _: &WlOutput) {}
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer_surface: &LayerSurface) {
        let before = self.surfaces.len();
        self.surfaces.retain(|s| s.layer_surface.wl_surface() != layer_surface.wl_surface());
        if self.surfaces.len() != before {
            tracing::info!("layer surface closed by compositor, releasing resources");
        }
    }

    fn configure(&mut self, _: &Connection, queue_handle: &QueueHandle<Self>, layer_surface: &LayerSurface, configure: LayerSurfaceConfigure, _: u32) {
        let Some(surface) = self.find_surface_mut(layer_surface.wl_surface()) else {
            return;
        };

        surface.configure(configure.new_size);
        self.render(queue_handle);
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

smithay_client_toolkit::delegate_registry!(State);
smithay_client_toolkit::delegate_dispatch2!(State);
