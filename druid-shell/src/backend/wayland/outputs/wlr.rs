use wayland_client as wlc;
use wayland_protocols::wlr::unstable::output_management::v1::client::zwlr_output_head_v1;
use wayland_protocols::wlr::unstable::output_management::v1::client::zwlr_output_manager_v1;
use wayland_protocols::wlr::unstable::output_management::v1::client::zwlr_output_mode_v1;

use super::super::error;
use super::super::outputs;

pub trait Consumer {
    fn consume<'a>(
        &'a self,
        obj: &'a wlc::Main<zwlr_output_head_v1::ZwlrOutputHeadV1>,
        event: &'a zwlr_output_head_v1::Event,
    );
}

#[derive(Default)]
struct Meta {
    meta: outputs::Meta,
    modes: Vec<std::sync::Arc<std::cell::RefCell<outputs::Mode>>>,
}

pub fn detect(
    registry: &wlc::GlobalManager,
) -> Result<calloop::channel::Channel<outputs::Event>, error::Error> {
    let (outputsaddedtx, outputsaddedrx) = calloop::channel::channel::<outputs::Event>();
    let zwlr_output_manager = registry
        .instantiate_exact::<zwlr_output_manager_v1::ZwlrOutputManagerV1>(2)
        .map_err(|e| error::Error::global("zxdg_output_manager_v1", 2, e))?;

    zwlr_output_manager.quick_assign({
        let mut outputs = Vec::<std::sync::Arc<std::cell::RefCell<Meta>>>::new();
        move |m, event, ctx| {
            tracing::debug!("global zwlr output manager {:?} {:?} {:?}", m, ctx, event);
            match event {
                zwlr_output_manager_v1::Event::Head { head } => {
                    tracing::debug!("zwlr_output_manager head event {:?} {:?}", m, head);
                    let current = std::sync::Arc::new(std::cell::RefCell::new(Meta::default()));
                    outputs.push(current.clone());
                    head.quick_assign(move |obj, event, _| {
                        Consumer::consume(&current, &obj, &event)
                    });
                }
                zwlr_output_manager_v1::Event::Done { .. } => {
                    for m in &outputs {
                        let m = m.borrow().meta.clone().normalize();
                        if let Err(cause) = outputsaddedtx.send(outputs::Event::Located(m)) {
                            tracing::error!("unable to deliver output event: {:?}", cause);
                        }
                    }
                }
                event => {
                    tracing::warn!("unhandled zwlr_output_manager event {:?} {:?}", m, event);
                }
            };
        }
    });

    zwlr_output_manager.create_configuration(0);

    Ok(outputsaddedrx)
}

impl Consumer for std::sync::Arc<std::cell::RefCell<Meta>> {
    fn consume(
        &self,
        obj: &wlc::Main<zwlr_output_head_v1::ZwlrOutputHeadV1>,
        event: &zwlr_output_head_v1::Event,
    ) {
        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                self.borrow_mut().meta.name = name.to_string();
            }
            zwlr_output_head_v1::Event::Description { description } => {
                self.borrow_mut().meta.description = description.to_string();
            }
            zwlr_output_head_v1::Event::PhysicalSize { width, height } => {
                self.borrow_mut().meta.physical = outputs::Dimensions::from((*width, *height));
            }
            zwlr_output_head_v1::Event::Make { make } => {
                self.borrow_mut().meta.make = make.to_string();
            }
            zwlr_output_head_v1::Event::Model { model } => {
                self.borrow_mut().meta.model = model.to_string();
            }
            zwlr_output_head_v1::Event::SerialNumber { .. } => {} // ignored
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                self.borrow_mut().meta.enabled = *enabled > 0;
            }
            zwlr_output_head_v1::Event::Position { x, y } => {
                self.borrow_mut().meta.position = outputs::Position::from((*x, *y));
            }
            zwlr_output_head_v1::Event::Scale { scale } => {
                self.borrow_mut().meta.scale = *scale;
            }
            zwlr_output_head_v1::Event::Transform { transform } => {
                self.borrow_mut().meta.transform = *transform;
            }
            zwlr_output_head_v1::Event::Mode { mode } => {
                let current =
                    std::sync::Arc::new(std::cell::RefCell::new(outputs::Mode::default()));
                self.borrow_mut().modes.push(current.clone());
                mode.quick_assign({
                    move |m, event, _ctx| match event {
                        zwlr_output_mode_v1::Event::Size { width, height } => {
                            current.borrow_mut().logical =
                                outputs::Dimensions::from((width, height));
                        }
                        zwlr_output_mode_v1::Event::Refresh { refresh } => {
                            current.borrow_mut().refresh = refresh;
                        }
                        zwlr_output_mode_v1::Event::Preferred => {
                            current.borrow_mut().preferred = true;
                        }
                        _ => tracing::debug!("unhandled mode event {:?} {:?}", m, event),
                    }
                });
            }
            zwlr_output_head_v1::Event::CurrentMode { mode: _ } => {
                // BUG: api here is pretty brutal. doesn't seem to be
                // a way to get a main object from the provided mode.
                // or to compare within the current set of modes for a match.
                // as a result we *incorrectly* just assign the preferred mode
                // as the current.
                let b = self.borrow();
                let mut modes = b.modes.iter();
                let mode = match modes.find(|m| m.borrow().preferred) {
                    Some(m) => m.borrow().clone(),
                    None => return,
                };
                drop(modes);
                drop(b);

                self.borrow_mut().meta.logical = mode.logical.clone();
                self.borrow_mut().meta.refresh = mode.refresh;
            }
            _ => tracing::warn!("unhandled {:?} {:?}", obj, event),
        };
    }
}
