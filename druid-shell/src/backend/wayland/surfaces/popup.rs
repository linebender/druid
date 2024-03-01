// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use wayland_client as wlc;
use wayland_protocols::xdg_shell::client::xdg_popup;
use wayland_protocols::xdg_shell::client::xdg_positioner;
use wayland_protocols::xdg_shell::client::xdg_surface;

use crate::kurbo;
use crate::window;

use super::error;
use super::surface;
use super::Compositor;
use super::CompositorHandle;
use super::Handle;
use super::Outputs;
use super::Popup;

#[allow(unused)]
struct Inner {
    wl_surface: surface::Surface,
    wl_xdg_surface: wlc::Main<xdg_surface::XdgSurface>,
    wl_xdg_popup: wlc::Main<xdg_popup::XdgPopup>,
    wl_xdg_pos: wlc::Main<xdg_positioner::XdgPositioner>,
}

impl From<Inner> for std::sync::Arc<surface::Data> {
    fn from(s: Inner) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.wl_surface)
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
        // requires version 3...
        // pos.set_reactive();

        pos
    }

    pub fn with_size(mut self, dim: kurbo::Size) -> Self {
        self.size = dim;
        self
    }

    pub fn with_offset(mut self, p: kurbo::Point) -> Self {
        self.offset = p;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: kurbo::Size::new(1., 1.),
            anchor: xdg_positioner::Anchor::Bottom,
            offset: kurbo::Point::ZERO,
            anchor_rect: (kurbo::Point::ZERO, kurbo::Size::from((1., 1.))),
            gravity: xdg_positioner::Gravity::BottomLeft,
            constraint_adjustment: xdg_positioner::ConstraintAdjustment::all(),
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
        parent: &dyn Popup,
    ) -> Result<Self, error::Error> {
        let compositor = CompositorHandle::new(c);
        let wl_surface = surface::Surface::new(compositor.clone(), handler, kurbo::Size::ZERO);
        let wl_xdg_surface = compositor.get_xdg_surface(&wl_surface.inner.wl_surface.borrow());

        // register to receive xdg_surface events.
        wl_xdg_surface.quick_assign({
            let wl_surface = wl_surface.clone();
            move |xdg_surface, event, _| match event {
                xdg_surface::Event::Configure { serial } => {
                    xdg_surface.ack_configure(serial);
                    let dim = wl_surface.inner.logical_size.get();
                    wl_surface.inner.handler.borrow_mut().size(dim);
                    wl_surface.request_paint();
                }
                _ => tracing::warn!("unhandled xdg_surface event {:?}", event),
            }
        });

        let wl_xdg_pos = config.apply(&compositor);
        wl_xdg_pos.quick_assign(|obj, event, _| {
            tracing::debug!("{:?} {:?}", obj, event);
        });

        let wl_xdg_popup = match parent.surface(&wl_xdg_surface, &wl_xdg_pos) {
            Ok(p) => p,
            Err(cause) => return Err(cause),
        };
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
                        tracing::debug!(
                            "popup configuration ({:?},{:?}) {:?}x{:?}",
                            x,
                            y,
                            width,
                            height
                        );
                        wl_surface.update_dimensions((width as f64, height as f64));
                    }
                    xdg_popup::Event::PopupDone => {
                        tracing::debug!("popup done {:?}", event);
                        match wl_surface.data() {
                            None => tracing::warn!("missing surface data, cannot close popup"),
                            Some(data) => {
                                data.with_handler(|winhandle| {
                                    winhandle.request_close();
                                });
                            }
                        };
                    }
                    _ => tracing::warn!("unhandled xdg_popup event configure {:?}", event),
                };
            }
        });

        let handle = Self {
            inner: std::sync::Arc::new(Inner {
                wl_surface,
                wl_xdg_surface,
                wl_xdg_popup,
                wl_xdg_pos,
            }),
        };

        handle.commit();
        Ok(handle)
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

impl Handle for Surface {
    fn get_size(&self) -> kurbo::Size {
        self.inner.wl_surface.get_size()
    }

    fn set_size(&self, dim: kurbo::Size) {
        self.inner.wl_surface.set_size(dim);
    }

    fn request_anim_frame(&self) {
        self.inner.wl_surface.request_anim_frame()
    }

    fn invalidate(&self) {
        self.inner.wl_surface.invalidate()
    }

    fn invalidate_rect(&self, rect: kurbo::Rect) {
        self.inner.wl_surface.invalidate_rect(rect)
    }

    fn remove_text_field(&self, token: crate::TextFieldToken) {
        self.inner.wl_surface.remove_text_field(token)
    }

    fn set_focused_text_field(&self, active_field: Option<crate::TextFieldToken>) {
        self.inner.wl_surface.set_focused_text_field(active_field)
    }

    fn set_input_region(&self, region: Option<crate::Region>) {
        self.inner.wl_surface.set_input_region(region)
    }

    fn get_idle_handle(&self) -> super::idle::Handle {
        self.inner.wl_surface.get_idle_handle()
    }

    fn get_scale(&self) -> crate::Scale {
        self.inner.wl_surface.get_scale()
    }

    fn run_idle(&self) {
        self.inner.wl_surface.run_idle();
    }

    fn release(&self) {
        self.inner.wl_surface.release()
    }

    fn data(&self) -> Option<std::sync::Arc<surface::Data>> {
        self.inner.wl_surface.data()
    }
}

impl From<&Surface> for std::sync::Arc<surface::Data> {
    fn from(s: &Surface) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.inner.wl_surface.clone())
    }
}

impl Popup for Surface {
    fn surface<'a>(
        &self,
        popup: &'a wlc::Main<xdg_surface::XdgSurface>,
        pos: &'a wlc::Main<wayland_protocols::xdg_shell::client::xdg_positioner::XdgPositioner>,
    ) -> Result<wlc::Main<wayland_protocols::xdg_shell::client::xdg_popup::XdgPopup>, error::Error>
    {
        Ok(popup.get_popup(Some(&self.inner.wl_xdg_surface), pos))
    }
}

impl From<Surface> for Box<dyn Outputs> {
    fn from(s: Surface) -> Box<dyn Outputs> {
        Box::new(s.inner.wl_surface.clone()) as Box<dyn Outputs>
    }
}

impl From<Surface> for Box<dyn Handle> {
    fn from(s: Surface) -> Box<dyn Handle> {
        Box::new(s) as Box<dyn Handle>
    }
}

impl From<Surface> for Box<dyn Popup> {
    fn from(s: Surface) -> Self {
        Box::new(s) as Box<dyn Popup>
    }
}
