use wayland_client as wlc;
use wayland_protocols::xdg_shell::client::xdg_popup;
use wayland_protocols::xdg_shell::client::xdg_positioner;
use wayland_protocols::xdg_shell::client::xdg_surface;

use crate::kurbo;
use crate::window;

use super::surface;
use super::Compositor;
use super::CompositorHandle;
use super::Handle;

struct Inner {
    wl_surface: surface::Handle,
    wl_xdg_popup: wlc::Main<xdg_popup::XdgPopup>,
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
pub struct Config {
    pub size: kurbo::Size,
    pub offset: kurbo::Point,
    pub anchor_rect: (kurbo::Point, kurbo::Size),
    pub anchor: xdg_positioner::Anchor,
    pub gravity: xdg_positioner::Gravity,
    pub constraint_adjustment: xdg_positioner::ConstraintAdjustment,
}

impl Config {
    fn apply(self, c: &CompositorHandle) -> wlc::Main<xdg_positioner::XdgPositioner> {
        tracing::debug!("configuring popup {:?}", self);
        let pos = c.get_xdg_positioner();

        pos.set_size(self.size.width as i32, self.size.height as i32);
        pos.set_offset(self.offset.x as i32, self.offset.y as i32);
        pos.set_anchor_rect(
            self.anchor_rect.0.x as i32,
            self.anchor_rect.0.y as i32,
            self.anchor_rect.1.width as i32,
            self.anchor_rect.1.height as i32,
        );
        pos.set_anchor(self.anchor);
        pos.set_gravity(self.gravity);
        pos.set_constraint_adjustment(self.constraint_adjustment.bits());

        pos
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: kurbo::Size::new(1., 1.),
            anchor: xdg_positioner::Anchor::None,
            offset: kurbo::Point::ZERO,
            anchor_rect: (kurbo::Point::ZERO, kurbo::Size::from((1., 1.))),
            gravity: xdg_positioner::Gravity::None,
            constraint_adjustment: xdg_positioner::ConstraintAdjustment::None,
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
        parent: Option<&xdg_surface::XdgSurface>,
    ) -> Self {
        let compositor = CompositorHandle::new(c);
        let wl_surface = surface::Handle::new(compositor.clone(), handler, kurbo::Size::ZERO);
        let wl_xdg_surface = compositor.get_xdg_surface(&wl_surface.inner.wl_surface);

        // register to receive xdg_surface events.
        wl_xdg_surface.quick_assign({
            let wl_surface = wl_surface.clone();
            move |xdg_surface, event, _| match event {
                xdg_surface::Event::Configure { serial } => {
                    xdg_surface.ack_configure(serial);
                    let dim = wl_surface.inner.logical_size.get();
                    wl_surface.inner.handler.borrow_mut().size(dim);
                    wl_surface.inner.buffers.request_paint(&wl_surface.inner);
                }
                _ => tracing::warn!("unhandled xdg_surface event {:?}", event),
            }
        });

        let pos = config.apply(&compositor);

        let wl_xdg_popup = wl_xdg_surface.get_popup(parent, &pos);

        pos.destroy();

        wl_xdg_popup.quick_assign({
            let wl_surface = wl_surface.clone();
            move |_xdg_popup, event, _| {
                match event {
                    xdg_popup::Event::Configure {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        tracing::trace!(
                            "popup configuration ({:?},{:?}) {:?}x{:?}",
                            x,
                            y,
                            width,
                            height
                        );
                        wl_surface.update_dimensions(width as u32, height as u32);
                    }
                    _ => tracing::warn!("unhandled xdg_popup event configure {:?}", event),
                };
            }
        });

        let handle = Self {
            inner: std::sync::Arc::new(Inner {
                wl_surface,
                wl_xdg_popup,
            }),
        };

        handle
    }

    pub(super) fn get_xdg_popup(&self) -> wlc::Main<xdg_popup::XdgPopup> {
        self.inner.wl_xdg_popup.clone()
    }

    pub(super) fn commit(&self) {
        let wl_surface = &self.inner.wl_surface;
        wl_surface.commit();
    }

    pub(crate) fn with_handler<T, F: FnOnce(&mut dyn window::WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        std::sync::Arc::<surface::Data>::from(self).with_handler(f)
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
