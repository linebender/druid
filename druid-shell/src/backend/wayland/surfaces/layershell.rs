use wayland_client as wlc;
use wayland_protocols::wlr::unstable::layer_shell::v1::client as layershell;

use crate::kurbo;
use crate::window;

use super::super::error;
use super::popup;
use super::surface;
use super::Compositor;
use super::CompositorHandle;
use super::Handle;
use super::Popup;
use super::PopupHandle;

struct Inner {
    wl_surface: surface::Handle,
    ls_surface: wlc::Main<layershell::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.ls_surface.destroy();
    }
}

impl Popup for Inner {
    fn popup_impl(&self, p: &popup::Surface) -> Result<(), error::Error> {
        tracing::info!("layershell get popup initiated");
        self.ls_surface.get_popup(&p.get_xdg_popup());
        p.commit();
        Ok(())
    }
}

impl From<Inner> for u32 {
    fn from(s: Inner) -> u32 {
        u32::from(s.wl_surface.clone())
    }
}

impl From<Inner> for std::sync::Arc<surface::Data> {
    fn from(s: Inner) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.wl_surface.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Margin {
    top: i32,
    right: i32,
    bottom: i32,
    left: i32,
}

impl Default for Margin {
    fn default() -> Self {
        Margin::from((0, 0, 0, 0))
    }
}

impl Margin {
    pub fn new(m: impl Into<Margin>) -> Self {
        m.into()
    }

    pub fn uniform(m: i32) -> Self {
        Margin::from((m, m, m, m))
    }
}

impl From<(i32, i32, i32, i32)> for Margin {
    fn from(margins: (i32, i32, i32, i32)) -> Self {
        Self {
            top: margins.0,
            left: margins.1,
            bottom: margins.2,
            right: margins.3,
        }
    }
}

impl From<i32> for Margin {
    fn from(m: i32) -> Self {
        Margin::from((m, m, m, m))
    }
}

impl From<(i32, i32)> for Margin {
    fn from(m: (i32, i32)) -> Self {
        Margin::from((m.0, m.1, m.0, m.1))
    }
}

#[derive(Clone)]
pub struct Config {
    pub initial_size: kurbo::Size,
    pub layer: layershell::zwlr_layer_shell_v1::Layer,
    pub keyboard_interactivity: layershell::zwlr_layer_surface_v1::KeyboardInteractivity,
    pub anchor: layershell::zwlr_layer_surface_v1::Anchor,
    pub exclusive_zone: i32,
    pub margin: Margin,
    pub namespace: &'static str,
    pub app_id: &'static str,
}

impl Config {
    pub fn keyboard_interactivity(
        mut self,
        mode: layershell::zwlr_layer_surface_v1::KeyboardInteractivity,
    ) -> Self {
        self.keyboard_interactivity = mode;
        self
    }

    pub fn layer(mut self, layer: layershell::zwlr_layer_shell_v1::Layer) -> Self {
        self.layer = layer;
        self
    }

    pub fn anchor(mut self, anchor: layershell::zwlr_layer_surface_v1::Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn margin(mut self, m: impl Into<Margin>) -> Self {
        self.margin = m.into();
        self
    }

    fn apply(self, surface: &Surface) {
        surface.initialize_dimensions(self.initial_size);
        surface
            .inner
            .ls_surface
            .set_exclusive_zone(self.exclusive_zone);
        surface.inner.ls_surface.set_anchor(self.anchor);
        surface
            .inner
            .ls_surface
            .set_keyboard_interactivity(self.keyboard_interactivity);
        surface.inner.ls_surface.set_margin(
            self.margin.top,
            self.margin.right,
            self.margin.bottom,
            self.margin.left,
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layer: layershell::zwlr_layer_shell_v1::Layer::Overlay,
            initial_size: kurbo::Size::ZERO,
            keyboard_interactivity: layershell::zwlr_layer_surface_v1::KeyboardInteractivity::None,
            anchor: layershell::zwlr_layer_surface_v1::Anchor::empty(),
            exclusive_zone: 0,
            margin: Margin::default(),
            namespace: "druid",
            app_id: "",
        }
    }
}

#[derive(Clone)]
pub struct Surface {
    inner: std::sync::Arc<Inner>,
}

impl Surface {
    pub fn new(
        c: impl Into<CompositorHandle>,
        handler: Box<dyn window::WinHandler>,
        config: Config,
    ) -> Self {
        let compositor = CompositorHandle::new(c);
        let wl_surface = surface::Handle::new(compositor.clone(), handler, kurbo::Size::ZERO);
        let ls_surface = compositor.zwlr_layershell_v1().get_layer_surface(
            &wl_surface.inner.wl_surface,
            None,
            config.layer,
            config.namespace.to_string(),
        );

        let handle = Self {
            inner: std::sync::Arc::new(Inner {
                wl_surface,
                ls_surface,
            }),
        };

        handle.inner.wl_surface.set_popup_impl(PopupHandle {
            inner: handle.inner.clone(),
        });
        handle.inner.ls_surface.quick_assign({
            let handle = handle.clone();
            let mut dim = config.initial_size.clone();
            move |a1, event, a2| match event {
                layershell::zwlr_layer_surface_v1::Event::Configure {
                    serial,
                    width,
                    height,
                } => {
                    tracing::info!("event {:?} {:?} {:?}", a1, event, a2);
                    // compositor is deferring to the client for determining the size
                    // when values are zero.
                    if width != 0 && height != 0 {
                        dim = kurbo::Size::new(width as f64, height as f64);
                    }

                    handle.inner.ls_surface.ack_configure(serial);
                    handle
                        .inner
                        .ls_surface
                        .set_size(dim.width as u32, dim.height as u32);
                    handle
                        .inner
                        .wl_surface
                        .update_dimensions(dim.width as u32, dim.height as u32);
                    handle.inner.wl_surface.inner.buffers.request_paint();
                }
                _ => tracing::info!("unimplemented event {:?} {:?} {:?}", a1, event, a2),
            }
        });

        config.apply(&handle);
        handle.inner.wl_surface.commit();

        handle
    }

    pub(crate) fn with_handler<T, F: FnOnce(&mut dyn window::WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        std::sync::Arc::<surface::Data>::from(self).with_handler(f)
    }

    fn initialize_dimensions(&self, dim: kurbo::Size) {
        self.inner
            .ls_surface
            .set_size(dim.width as u32, dim.height as u32);
        self.inner.wl_surface.inner.handler.borrow_mut().size(dim);
    }
}

impl From<Surface> for u32 {
    fn from(s: Surface) -> u32 {
        u32::from(&s.inner.wl_surface)
    }
}

impl From<&Surface> for u32 {
    fn from(s: &Surface) -> u32 {
        u32::from(&s.inner.wl_surface)
    }
}

impl From<&Surface> for std::sync::Arc<surface::Data> {
    fn from(s: &Surface) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.inner.wl_surface.clone())
    }
}

impl From<Surface> for std::sync::Arc<surface::Data> {
    fn from(s: Surface) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.inner.wl_surface.clone())
    }
}

impl From<Surface> for Box<dyn Handle> {
    fn from(s: Surface) -> Box<dyn Handle> {
        Box::new(s.inner.wl_surface.clone()) as Box<dyn Handle>
    }
}
