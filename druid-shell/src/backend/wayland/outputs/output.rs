// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use super::super::display;
use super::super::error;
use super::super::outputs;
use wayland_client as wlc;
use wayland_client::protocol::wl_output;
use wayland_client::protocol::wl_registry;
use wayland_protocols::unstable::xdg_output::v1::client::zxdg_output_manager_v1;
use wayland_protocols::unstable::xdg_output::v1::client::zxdg_output_v1;

pub fn detect(
    env: &impl display::GlobalEventDispatch,
) -> Result<calloop::channel::Channel<outputs::Event>, error::Error> {
    let (outputstx, outputsrx) = calloop::channel::channel::<outputs::Event>();
    let xdg_output_manager_id: std::cell::RefCell<Option<u32>> = std::cell::RefCell::new(None);
    display::GlobalEventDispatch::subscribe(env, {
        move |event: &'_ wlc::GlobalEvent,
              registry: &'_ wlc::Attached<wl_registry::WlRegistry>,
              _ctx: &'_ wlc::DispatchData| {
            match event {
                wlc::GlobalEvent::New {
                    id,
                    interface,
                    version,
                } => {
                    let id = *id;
                    let version = *version;

                    if interface.as_str() == "zxdg_output_manager_v1" && version == 3 {
                        xdg_output_manager_id.replace(Some(id));
                        return;
                    }

                    // We rely on wl_output::done() event so version 2 is our minimum,
                    // it is also 9 years old so we can assume that any non-abandonware compositor uses it.
                    if !(interface.as_str() == "wl_output" && version >= 2) {
                        return;
                    }

                    let version = version.min(3);
                    let output = registry.bind::<wl_output::WlOutput>(version, id);
                    let xdgm = (*xdg_output_manager_id.borrow()).map(|xdgm_id| {
                        registry.bind::<zxdg_output_manager_v1::ZxdgOutputManagerV1>(3, xdgm_id)
                    });

                    let mut meta = Meta::default();
                    let mut xdgmeta = XdgMeta::new();
                    output.quick_assign({
                        let outputstx = outputstx.clone();
                        move |output, event, _ctx| {
                            let mut m = match meta.consume(&output, &event) {
                                Some(m) => m,
                                None => return,
                            };

                            if !xdgmeta.set_xdg_handled() {
                                if let Some(xdgm) = &xdgm {
                                    let xdg_output = xdgm.get_xdg_output(&output);
                                    xdg_output.quick_assign({
                                        let mut xdgmeta = xdgmeta.clone();
                                        move |xdg_output, event, _ctx| {
                                            xdgmeta.consume(&xdg_output, &event);
                                        }
                                    });
                                    return;
                                }
                            }

                            xdgmeta.modify(&mut m);
                            m.output = Some(output.detach());

                            if let Err(cause) = outputstx.send(outputs::Event::Located(m)) {
                                tracing::warn!("unable to transmit output {:?}", cause);
                            }
                        }
                    });
                }
                wlc::GlobalEvent::Removed { interface, .. } => {
                    if interface.as_str() != "wl_output" {
                        return;
                    }
                    tracing::debug!("output removed event {:?} {:?}", registry, interface);
                }
            };
        }
    });

    Ok(outputsrx)
}

#[derive(Debug, Default)]
struct XdgState {
    name: String,
    description: String,
    position: outputs::Position,
    logical: outputs::Dimensions,
}

#[derive(Clone, Debug)]
struct XdgMeta {
    handled: bool,
    state: std::sync::Arc<std::cell::RefCell<XdgState>>,
}

impl XdgMeta {
    fn new() -> Self {
        Self {
            handled: false,
            state: std::sync::Arc::new(std::cell::RefCell::new(XdgState::default())),
        }
    }

    fn set_xdg_handled(&mut self) -> bool {
        let tmp = self.handled;
        self.handled = true;
        tmp
    }

    fn consume(
        &mut self,
        output: &wlc::Main<zxdg_output_v1::ZxdgOutputV1>,
        evt: &zxdg_output_v1::Event,
    ) {
        match evt {
            zxdg_output_v1::Event::Name { name } => {
                self.state.borrow_mut().name = name.clone();
            }
            zxdg_output_v1::Event::Description { description } => {
                self.state.borrow_mut().description = description.clone();
            }
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                self.state.borrow_mut().position = outputs::Position::from((*x, *y));
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                self.state.borrow_mut().logical = outputs::Dimensions::from((*width, *height));
            }
            _ => tracing::warn!("unused xdg_output_v1 event {:?} {:?}", output, evt),
        };
    }

    fn modify(&self, meta: &mut outputs::Meta) {
        if !self.handled {
            return;
        }

        let state = self.state.borrow();
        meta.name = state.name.clone();
        meta.description = state.description.clone();
        meta.position = state.position.clone();
        meta.logical = state.logical.clone();
    }
}

#[derive(Default)]
struct Meta {
    meta: outputs::Meta,
}

impl Meta {
    /// Incorporate update data from the server for this output.
    fn consume(
        &mut self,
        output: &wlc::Main<wl_output::WlOutput>,
        evt: &wl_output::Event,
    ) -> Option<outputs::Meta> {
        match evt {
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {
                self.meta.position = outputs::Position::from((*x, *y));
                self.meta.physical = outputs::Dimensions::from((*physical_width, *physical_height));
                self.meta.subpixel = *subpixel;
                self.meta.make = make.clone();
                self.meta.model = model.clone();
                self.meta.transform = *transform;
                None
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                if flags.contains(wl_output::Mode::Current) {
                    self.meta.logical = outputs::Dimensions::from((*width, *height));
                    self.meta.refresh = *refresh;
                }

                None
            }
            wl_output::Event::Done => {
                self.meta.gid = wlc::Proxy::from(output.detach()).id();
                self.meta.enabled = true;
                Some(self.meta.clone())
            }
            wl_output::Event::Scale { factor } => {
                self.meta.scale = (*factor).into();
                None
            }
            _ => {
                tracing::warn!("unknown output event {:?}", evt); // ignore possible future events
                None
            }
        }
    }
}
