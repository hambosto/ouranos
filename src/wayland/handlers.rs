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

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, wl_output: WlOutput) {
        self.surfaces.retain(|s| {
            if s.output == wl_output {
                s.destroy();
                false
            } else {
                true
            }
        });
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

    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _: Transform) {}

    fn frame(&mut self, _: &Connection, queue_handle: &QueueHandle<Self>, wl_surface: &WlSurface, _: u32) {
        let Some(surface) = self.find_surface_mut(wl_surface) else {
            return;
        };

        if let Err(e) = surface.tick(queue_handle) {
            tracing::warn!(?e, "failed to tick transition");
        }
    }

    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _: &WlOutput) {}

    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlSurface, _: &WlOutput) {}
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer_surface: &LayerSurface) {
        let wl_surface = layer_surface.wl_surface();
        self.surfaces.retain(|s| s.layer_surface.wl_surface() != wl_surface);
        tracing::info!("layer surface closed by compositor, releasing resources");
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
