use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::{self as wlc, protocol::wl_surface::WlSurface};
use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols::xdg_shell::client::xdg_popup;
use wayland_protocols::xdg_shell::client::xdg_positioner;
use wayland_protocols::xdg_shell::client::xdg_surface;

use crate::kurbo;
use crate::Scale;
use crate::TextFieldToken;

use super::application;
use super::error;

pub mod buffers;
pub mod idle;
pub mod layershell;
pub mod popup;
pub mod surface;
pub mod toplevel;

pub static GLOBAL_ID: crate::Counter = crate::Counter::new();

pub trait Compositor {
    fn output(&self, id: &u32) -> Option<application::Output>;
    fn create_surface(&self) -> wlc::Main<WlSurface>;
    fn shared_mem(&self) -> wlc::Main<WlShm>;
    fn get_xdg_surface(&self, surface: &wlc::Main<WlSurface>)
        -> wlc::Main<xdg_surface::XdgSurface>;
    fn get_xdg_positioner(&self) -> wlc::Main<xdg_positioner::XdgPositioner>;
    fn zxdg_decoration_manager_v1(&self) -> wlc::Main<ZxdgDecorationManagerV1>;
    fn zwlr_layershell_v1(&self) -> wlc::Main<ZwlrLayerShellV1>;
}

pub trait Decor {
    fn inner_set_title(&self, title: String);
}

impl dyn Decor {
    pub fn set_title(&self, title: impl Into<String>) {
        self.inner_set_title(title.into())
    }
}

pub trait Popup {
    fn surface<'a>(
        &self,
        popup: &'a wlc::Main<xdg_surface::XdgSurface>,
        pos: &'a wlc::Main<xdg_positioner::XdgPositioner>,
    ) -> Result<wlc::Main<xdg_popup::XdgPopup>, error::Error>;
}

pub(super) trait Outputs {
    fn removed(&self, o: &application::Output);
    fn inserted(&self, o: &application::Output);
}

// handle on given surface.
pub trait Handle {
    fn get_size(&self) -> kurbo::Size;
    fn set_size(&self, dim: kurbo::Size);
    fn request_anim_frame(&self);
    fn invalidate(&self);
    fn invalidate_rect(&self, rect: kurbo::Rect);
    fn remove_text_field(&self, token: TextFieldToken);
    fn set_focused_text_field(&self, active_field: Option<TextFieldToken>);
    fn get_idle_handle(&self) -> idle::Handle;
    fn get_scale(&self) -> Scale;
    fn run_idle(&self);
    fn release(&self);
    fn data(&self) -> Option<std::sync::Arc<surface::Data>>;
}

#[derive(Clone)]
pub struct CompositorHandle {
    inner: std::sync::Weak<dyn Compositor>,
}

impl CompositorHandle {
    pub fn new(c: impl Into<CompositorHandle>) -> Self {
        c.into()
    }

    pub fn direct(c: std::sync::Weak<dyn Compositor>) -> Self {
        Self { inner: c }
    }

    fn create_surface(&self) -> Option<wlc::Main<WlSurface>> {
        match self.inner.upgrade() {
            Some(c) => Some(c.create_surface()),
            None => None,
        }
    }

    /// Recompute the scale to use (the maximum of all the provided outputs).
    fn recompute_scale<'a>(&self, outputs: &'a std::collections::HashSet<u32>) -> i32 {
        let compositor = match self.inner.upgrade() {
            Some(c) => c,
            None => panic!("should never recompute scale of window that has been dropped"),
        };
        tracing::debug!("computing scale using {:?} outputs", outputs.len());
        let scale = outputs.iter().fold(0, |scale, id| {
            tracing::debug!("recomputing scale using output {:?}", id);
            match compositor.output(id) {
                None => {
                    tracing::warn!(
                        "we still have a reference to an output that's gone away. The output had id {}",
                        id,
                    );
                    scale
                },
                Some(output) => scale.max(output.scale),
            }
        });

        match scale {
            0 => {
                tracing::warn!("wayland never reported which output we are drawing to");
                1
            }
            scale => scale,
        }
    }
}

impl Compositor for CompositorHandle {
    fn output(&self, id: &u32) -> Option<application::Output> {
        match self.inner.upgrade() {
            None => None,
            Some(c) => c.output(id),
        }
    }

    fn create_surface(&self) -> wlc::Main<WlSurface> {
        match self.inner.upgrade() {
            None => panic!("unable to acquire underyling compositor to create a surface"),
            Some(c) => c.create_surface(),
        }
    }

    fn shared_mem(&self) -> wlc::Main<WlShm> {
        match self.inner.upgrade() {
            None => panic!("unable to acquire underyling compositor to acquire shared memory"),
            Some(c) => c.shared_mem(),
        }
    }

    fn get_xdg_positioner(&self) -> wlc::Main<xdg_positioner::XdgPositioner> {
        match self.inner.upgrade() {
            None => panic!("unable to acquire underyling compositor to create an xdg positioner"),
            Some(c) => c.get_xdg_positioner(),
        }
    }

    fn get_xdg_surface(&self, s: &wlc::Main<WlSurface>) -> wlc::Main<xdg_surface::XdgSurface> {
        match self.inner.upgrade() {
            None => panic!("unable to acquire underyling compositor to create an xdg surface"),
            Some(c) => c.get_xdg_surface(s),
        }
    }

    fn zxdg_decoration_manager_v1(&self) -> wlc::Main<ZxdgDecorationManagerV1> {
        match self.inner.upgrade() {
            None => {
                panic!("unable to acquire underyling compositor to acquire the decoration manager")
            }
            Some(c) => c.zxdg_decoration_manager_v1(),
        }
    }

    fn zwlr_layershell_v1(&self) -> wlc::Main<ZwlrLayerShellV1> {
        match self.inner.upgrade() {
            None => {
                panic!("unable to acquire underyling compositor to acquire the layershell manager")
            }
            Some(c) => c.zwlr_layershell_v1(),
        }
    }
}
