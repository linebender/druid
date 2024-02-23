// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use wayland_client as wlc;
use wayland_protocols::wlr::unstable::layer_shell::v1::client as layershell;
use wayland_protocols::xdg_shell::client::xdg_surface;

use crate::kurbo;
use crate::window;

use super::super::error;
use super::super::outputs;
use super::surface;
use super::Compositor;
use super::CompositorHandle;
use super::Handle;
use super::Outputs;
use super::Popup;

#[derive(Default)]
struct Output {
    preferred: Option<String>,
    current: Option<outputs::Meta>,
}

struct Inner {
    config: Config,
    wl_surface: std::cell::RefCell<surface::Surface>,
    ls_surface:
        std::cell::RefCell<wlc::Main<layershell::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>>,
    requires_initialization: std::cell::RefCell<bool>,
    available: std::cell::RefCell<bool>,
    output: std::cell::RefCell<Output>,
}

impl Inner {
    fn popup<'a>(
        &self,
        surface: &'a wlc::Main<xdg_surface::XdgSurface>,
        pos: &'a wlc::Main<wayland_protocols::xdg_shell::client::xdg_positioner::XdgPositioner>,
    ) -> wlc::Main<wayland_protocols::xdg_shell::client::xdg_popup::XdgPopup> {
        let popup = surface.get_popup(None, pos);
        self.ls_surface.borrow().get_popup(&popup);
        popup
    }
}

impl Popup for Inner {
    fn surface<'a>(
        &self,
        surface: &'a wlc::Main<xdg_surface::XdgSurface>,
        pos: &'a wlc::Main<wayland_protocols::xdg_shell::client::xdg_positioner::XdgPositioner>,
    ) -> Result<wlc::Main<wayland_protocols::xdg_shell::client::xdg_popup::XdgPopup>, error::Error>
    {
        Ok(self.popup(surface, pos))
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.ls_surface.borrow().destroy();
    }
}

impl From<Inner> for std::sync::Arc<surface::Data> {
    fn from(s: Inner) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.wl_surface.borrow().clone())
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

    fn apply(&self, surface: &Surface) {
        let ls = surface.inner.ls_surface.borrow();
        ls.set_exclusive_zone(self.exclusive_zone);
        ls.set_anchor(self.anchor);
        ls.set_keyboard_interactivity(self.keyboard_interactivity);
        ls.set_margin(
            self.margin.top,
            self.margin.right,
            self.margin.bottom,
            self.margin.left,
        );
        ls.set_size(
            self.initial_size.width as u32,
            self.initial_size.height as u32,
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layer: layershell::zwlr_layer_shell_v1::Layer::Overlay,
            initial_size: kurbo::Size::ZERO,
            keyboard_interactivity: layershell::zwlr_layer_surface_v1::KeyboardInteractivity::None,
            anchor: layershell::zwlr_layer_surface_v1::Anchor::all(),
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
        let wl_surface = surface::Surface::new(compositor.clone(), handler, kurbo::Size::ZERO);
        let ls_surface = compositor.zwlr_layershell_v1().unwrap().get_layer_surface(
            &wl_surface.inner.wl_surface.borrow(),
            None,
            config.layer,
            config.namespace.to_string(),
        );

        let handle = Self {
            inner: std::sync::Arc::new(Inner {
                config,
                wl_surface: std::cell::RefCell::new(wl_surface),
                ls_surface: std::cell::RefCell::new(ls_surface),
                requires_initialization: std::cell::RefCell::new(true),
                available: std::cell::RefCell::new(false),
                output: std::cell::RefCell::new(Default::default()),
            }),
        };

        Surface::initialize(&handle);
        handle
    }

    pub(crate) fn with_handler<T, F: FnOnce(&mut dyn window::WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        std::sync::Arc::<surface::Data>::from(self).with_handler(f)
    }

    fn initialize(handle: &Surface) {
        handle.inner.requires_initialization.replace(false);
        tracing::debug!("attempting to initialize layershell");

        handle.inner.ls_surface.borrow().quick_assign({
            let handle = handle.clone();
            move |a1, event, a2| {
                tracing::debug!("consuming event {:?} {:?} {:?}", a1, event, a2);
                Surface::consume_layershell_event(&handle, &a1, &event, &a2);
            }
        });

        handle.inner.config.apply(handle);
        handle.inner.wl_surface.borrow().commit();
    }

    fn consume_layershell_event(
        handle: &Surface,
        a1: &wlc::Main<layershell::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
        event: &layershell::zwlr_layer_surface_v1::Event,
        data: &wlc::DispatchData,
    ) {
        match *event {
            layershell::zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                let mut dim = handle.inner.config.initial_size;
                // compositor is deferring to the client for determining the size
                // when values are zero.
                if width != 0 && height != 0 {
                    dim = kurbo::Size::new(width as f64, height as f64);
                }

                let ls = handle.inner.ls_surface.borrow();
                ls.ack_configure(serial);
                ls.set_size(dim.width as u32, dim.height as u32);
                handle.inner.wl_surface.borrow().update_dimensions(dim);
                handle.inner.wl_surface.borrow().request_paint();
                handle.inner.available.replace(true);
            }
            layershell::zwlr_layer_surface_v1::Event::Closed => {
                if let Some(o) = handle.inner.wl_surface.borrow().output() {
                    handle
                        .inner
                        .output
                        .borrow_mut()
                        .preferred
                        .get_or_insert(o.name.clone());
                    handle.inner.output.borrow_mut().current.get_or_insert(o);
                }
                handle.inner.ls_surface.borrow().destroy();
                handle.inner.available.replace(false);
                handle.inner.requires_initialization.replace(true);
            }
            _ => tracing::warn!("unimplemented event {:?} {:?} {:?}", a1, event, data),
        }
    }
}

impl Outputs for Surface {
    fn removed(&self, o: &outputs::Meta) {
        self.inner.wl_surface.borrow().removed(o);
        self.inner.output.borrow_mut().current.take();
    }

    fn inserted(&self, o: &outputs::Meta) {
        let old = String::from(
            self.inner
                .output
                .borrow()
                .preferred
                .as_ref()
                .map_or("", |name| name),
        );

        let reinitialize = *self.inner.requires_initialization.borrow();
        let reinitialize = old == o.name || reinitialize;
        if !reinitialize {
            tracing::debug!(
                "skipping reinitialization output for layershell {:?} {:?} == {:?} || {:?} -> {:?}",
                o.id(),
                o.name,
                old,
                *self.inner.requires_initialization.borrow(),
                reinitialize,
            );
            return;
        }

        tracing::debug!(
            "reinitializing output for layershell {:?} {:?} == {:?} || {:?} -> {:?}",
            o.id(),
            o.name,
            old,
            *self.inner.requires_initialization.borrow(),
            reinitialize,
        );

        let sdata = self.inner.wl_surface.borrow().inner.clone();
        self.inner
            .wl_surface
            .replace(surface::Surface::replace(&sdata));
        let sdata = self.inner.wl_surface.borrow().inner.clone();
        let replacedlayershell = self.inner.ls_surface.replace(
            sdata
                .compositor
                .zwlr_layershell_v1()
                .unwrap()
                .get_layer_surface(
                    &self.inner.wl_surface.borrow().inner.wl_surface.borrow(),
                    o.output.as_ref(),
                    self.inner.config.layer,
                    self.inner.config.namespace.to_string(),
                ),
        );

        Surface::initialize(self);

        replacedlayershell.destroy();
    }
}

impl Popup for Surface {
    fn surface<'a>(
        &self,
        popup: &'a wlc::Main<xdg_surface::XdgSurface>,
        pos: &'a wlc::Main<wayland_protocols::xdg_shell::client::xdg_positioner::XdgPositioner>,
    ) -> Result<wlc::Main<wayland_protocols::xdg_shell::client::xdg_popup::XdgPopup>, error::Error>
    {
        Ok(self.inner.popup(popup, pos))
    }
}

impl Handle for Surface {
    fn get_size(&self) -> kurbo::Size {
        return self.inner.wl_surface.borrow().get_size();
    }

    fn set_size(&self, dim: kurbo::Size) {
        return self.inner.wl_surface.borrow().set_size(dim);
    }

    fn request_anim_frame(&self) {
        if *self.inner.available.borrow() {
            self.inner.wl_surface.borrow().request_anim_frame()
        }
    }

    fn invalidate(&self) {
        return self.inner.wl_surface.borrow().invalidate();
    }

    fn invalidate_rect(&self, rect: kurbo::Rect) {
        return self.inner.wl_surface.borrow().invalidate_rect(rect);
    }

    fn remove_text_field(&self, token: crate::TextFieldToken) {
        return self.inner.wl_surface.borrow().remove_text_field(token);
    }

    fn set_focused_text_field(&self, active_field: Option<crate::TextFieldToken>) {
        return self
            .inner
            .wl_surface
            .borrow()
            .set_focused_text_field(active_field);
    }

    fn set_input_region(&self, region: Option<crate::Region>) {
        self.inner.wl_surface.borrow().set_input_region(region);
    }

    fn get_idle_handle(&self) -> super::idle::Handle {
        return self.inner.wl_surface.borrow().get_idle_handle();
    }

    fn get_scale(&self) -> crate::Scale {
        return self.inner.wl_surface.borrow().get_scale();
    }

    fn run_idle(&self) {
        if *self.inner.available.borrow() {
            self.inner.wl_surface.borrow().run_idle();
        }
    }

    fn release(&self) {
        self.inner.wl_surface.borrow().release()
    }

    fn data(&self) -> Option<std::sync::Arc<surface::Data>> {
        self.inner.wl_surface.borrow().data()
    }
}

impl From<&Surface> for std::sync::Arc<surface::Data> {
    fn from(s: &Surface) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.inner.wl_surface.borrow().clone())
    }
}

impl From<Surface> for std::sync::Arc<surface::Data> {
    fn from(s: Surface) -> std::sync::Arc<surface::Data> {
        std::sync::Arc::<surface::Data>::from(s.inner.wl_surface.borrow().clone())
    }
}

impl From<Surface> for Box<dyn Handle> {
    fn from(s: Surface) -> Box<dyn Handle> {
        Box::new(s) as Box<dyn Handle>
    }
}

impl From<Surface> for Box<dyn Outputs> {
    fn from(s: Surface) -> Box<dyn Outputs> {
        Box::new(s) as Box<dyn Outputs>
    }
}

impl From<Surface> for Box<dyn Popup> {
    fn from(s: Surface) -> Self {
        Box::new(s) as Box<dyn Popup>
    }
}
