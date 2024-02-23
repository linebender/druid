// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use wayland_client as wlc;
use wayland_protocols::xdg_shell::client::xdg_surface;
use wayland_protocols::xdg_shell::client::xdg_toplevel;

use crate::kurbo;
use crate::window;

use super::error;
use super::surface;
use super::Compositor;
use super::CompositorHandle;
use super::Decor;
use super::Handle;
use super::Outputs;
use super::Popup;

struct Inner {
    wl_surface: surface::Surface,
    pub(super) xdg_surface: wlc::Main<xdg_surface::XdgSurface>,
    pub(super) xdg_toplevel: wlc::Main<xdg_toplevel::XdgToplevel>,
}

impl From<Inner> for std::sync::Arc<surface::Data> {
    fn from(s: Inner) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.wl_surface)
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
        size: kurbo::Size,
        min_size: Option<kurbo::Size>,
    ) -> Self {
        let min_size = min_size.unwrap_or_else(|| kurbo::Size::from((1.0, 1.0)));
        let compositor = CompositorHandle::new(c);
        let wl_surface = surface::Surface::new(compositor.clone(), handler, kurbo::Size::ZERO);
        let xdg_surface = compositor.get_xdg_surface(&wl_surface.inner.wl_surface.borrow());
        let xdg_toplevel = xdg_surface.get_toplevel();

        // register to receive xdg_surface events.
        xdg_surface.quick_assign({
            let wl_surface = wl_surface.clone();
            move |xdg_surface, event, _| {
                tracing::debug!("xdg_surface event configure {:?}", event);
                match event {
                    xdg_surface::Event::Configure { serial } => {
                        xdg_surface.ack_configure(serial);
                        wl_surface.resize(wl_surface.get_size());
                        wl_surface.request_paint();
                    }
                    _ => tracing::warn!("unhandled xdg_surface event {:?}", event),
                }
            }
        });

        xdg_toplevel.quick_assign({
            let wl_surface = wl_surface.clone();
            move |_xdg_toplevel, event, a3| match event {
                xdg_toplevel::Event::Configure {
                    width,
                    height,
                    states,
                } => {
                    tracing::debug!(
                        "configure event {:?} {:?} {:?} {:?}",
                        width,
                        height,
                        states,
                        a3
                    );

                    // If the width or height arguments are zero, it means the client should decide its own window dimension.
                    // This may happen when the compositor needs to configure the state of the surface
                    // but doesn't have any information about any previous or expected dimension.
                    let (width, height) = if width == 0 || height == 0 {
                        (size.width, size.height)
                    } else {
                        (width as f64, height as f64)
                    };

                    let dim =
                        kurbo::Size::new(width.max(min_size.width), height.max(min_size.height));

                    wl_surface.update_dimensions(dim);
                }
                xdg_toplevel::Event::Close => {
                    tracing::info!("xdg close event {:?}", event);
                    wl_surface.inner.handler.borrow_mut().request_close();
                }
                _ => tracing::info!("unimplemented event {:?}", event),
            }
        });

        let inner = Inner {
            wl_surface,
            xdg_toplevel,
            xdg_surface,
        };

        inner
            .xdg_toplevel
            .set_min_size(min_size.width as i32, min_size.height as i32);
        inner.xdg_toplevel.set_maximized();

        let handle = Self {
            inner: std::sync::Arc::new(inner),
        };

        handle.commit();
        handle
    }

    pub(crate) fn with_handler<T, F: FnOnce(&mut dyn window::WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        std::sync::Arc::<surface::Data>::from(self).with_handler(f)
    }

    pub(crate) fn commit(&self) {
        self.inner.wl_surface.commit();
    }
}

impl Popup for Surface {
    fn surface<'a>(
        &self,
        popup: &'a wlc::Main<xdg_surface::XdgSurface>,
        pos: &'a wlc::Main<wayland_protocols::xdg_shell::client::xdg_positioner::XdgPositioner>,
    ) -> Result<wlc::Main<wayland_protocols::xdg_shell::client::xdg_popup::XdgPopup>, error::Error>
    {
        Ok(popup.get_popup(Some(&self.inner.xdg_surface), pos))
    }
}

impl Decor for Surface {
    fn inner_set_title(&self, title: String) {
        self.inner.xdg_toplevel.set_title(title);
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

impl From<Surface> for Box<dyn Decor> {
    fn from(s: Surface) -> Box<dyn Decor> {
        Box::new(s) as Box<dyn Decor>
    }
}

impl From<Surface> for Box<dyn Outputs> {
    fn from(s: Surface) -> Box<dyn Outputs> {
        Box::new(s.inner.wl_surface.clone()) as Box<dyn Outputs>
    }
}

impl From<Surface> for Box<dyn Popup> {
    fn from(s: Surface) -> Self {
        Box::new(s) as Box<dyn Popup>
    }
}
