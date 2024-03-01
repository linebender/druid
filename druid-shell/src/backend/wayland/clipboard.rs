// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactions with the system pasteboard on wayland compositors.
use super::application;
use super::error as waylanderr;
use crate::clipboard::{ClipboardFormat, FormatId};
use std::io::Read;
use wayland_client as wl;
use wayland_client::protocol::wl_data_device;
use wayland_client::protocol::wl_data_device_manager;
use wayland_client::protocol::wl_data_offer;
use wayland_client::protocol::wl_data_source;

#[derive(Clone)]
struct Offer {
    wobj: wl::Main<wl_data_offer::WlDataOffer>,
    mimetype: String,
}

impl Offer {
    fn new(d: wl::Main<wl_data_offer::WlDataOffer>, mimetype: impl Into<String>) -> Self {
        Self {
            wobj: d,
            mimetype: mimetype.into(),
        }
    }
}

#[derive(Default)]
struct Data {
    pending: std::cell::RefCell<Vec<Offer>>,
    current: std::cell::RefCell<Vec<Offer>>,
}

impl Data {
    fn receive(&self, mimetype: &str) -> Option<Offer> {
        for offer in self.current.borrow().iter() {
            if !offer.mimetype.starts_with(mimetype) {
                continue;
            }
            return Some(offer.clone());
        }
        None
    }
}

impl std::fmt::Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data")
            .field("pending", &self.pending.borrow().len())
            .field("current", &self.current.borrow().len())
            .finish()
    }
}

impl From<Vec<Offer>> for Data {
    fn from(current: Vec<Offer>) -> Self {
        Self {
            current: std::cell::RefCell::new(current),
            pending: Default::default(),
        }
    }
}

struct Inner {
    display: wl::Display,
    wobj: wl::Main<wl_data_device_manager::WlDataDeviceManager>,
    wdsobj: wl::Main<wl_data_source::WlDataSource>,
    devices: std::rc::Rc<std::cell::RefCell<Data>>,
}

impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("wobj", &self.wobj)
            .field("wdsobj", &self.wdsobj)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct Manager {
    inner: std::rc::Rc<Inner>,
}

impl Manager {
    pub(super) fn new(
        display: &wl::Display,
        gm: &wl::GlobalManager,
    ) -> Result<Self, waylanderr::Error> {
        let m = gm
            .instantiate_exact::<wl_data_device_manager::WlDataDeviceManager>(3)
            .map_err(|e| waylanderr::Error::global("wl_data_device_manager", 1, e))?;

        m.quick_assign(|i, event, _ignored| {
            tracing::info!("clipboard {:?} event {:?}", i, event);
        });

        let ds = m.create_data_source();
        ds.quick_assign(|i, event, _ignored| {
            tracing::info!("clipboard {:?} event {:?}", i, event);
        });

        Ok(Self {
            inner: std::rc::Rc::new(Inner {
                wobj: m,
                wdsobj: ds,
                display: display.clone(),
                devices: Default::default(),
            }),
        })
    }

    pub fn attach<'a>(&'a self, seat: &'a mut application::Seat) {
        let device = self.inner.wobj.get_data_device(&seat.wl_seat);
        device.quick_assign({
            let m = self.inner.clone();
            move |i, event, _ignored| match event {
                wl_data_device::Event::DataOffer { id } => {
                    let offer = id;
                    offer.quick_assign({
                        let m = m.clone();
                        move |i, event, _ignored| match event {
                            wl_data_offer::Event::Offer { mime_type } => {
                                let data = m.devices.borrow_mut();
                                let offer = Offer::new(i, mime_type);
                                data.pending.borrow_mut().push(offer);
                            }
                            _ => tracing::warn!("clipboard unhandled {:?} event {:?}", i, event),
                        }
                    });
                }
                wl_data_device::Event::Selection { id } => {
                    if id.is_some() {
                        let data = m.devices.borrow();
                        tracing::debug!(
                            "current data offers {:?} {:?}",
                            data.current.borrow().len(),
                            data.pending.borrow().len()
                        );
                        let upd = Data::from(data.pending.take());
                        drop(data);
                        tracing::debug!(
                            "updated data offers {:?} {:?}",
                            upd.current.borrow().len(),
                            upd.pending.borrow().len()
                        );
                        m.devices.replace(upd);
                    } else {
                        let upd = Data::from(Vec::new());
                        m.devices.replace(upd);
                    }
                }
                _ => tracing::warn!("clipboard unhandled {:?} event {:?}", i, event),
            }
        });
    }

    fn initiate(&self, o: Offer) -> Option<Vec<u8>> {
        tracing::debug!("retrieving {:?} {:?}", o.wobj, o.mimetype);
        let (fdread, fdwrite) = match nix::unistd::pipe2(nix::fcntl::OFlag::O_CLOEXEC) {
            Ok(pipe) => pipe,
            Err(cause) => {
                tracing::error!("clipboard failed to request data {:?}", cause);
                return None;
            }
        };

        o.wobj.receive(o.mimetype.to_string(), fdwrite);
        if let Err(cause) = self.inner.display.flush() {
            tracing::error!("clipboard failed to request data {:?}", cause);
            return None;
        }

        if let Err(cause) = nix::unistd::close(fdwrite) {
            tracing::error!("clipboard failed to request data {:?}", cause);
            return None;
        }

        let mut data = Vec::new();
        let mut io: std::fs::File = unsafe { std::os::unix::io::FromRawFd::from_raw_fd(fdread) };
        let transferred = match io.read_to_end(&mut data) {
            Err(cause) => {
                tracing::error!("clipboard unable to retrieve pasted content {:?}", cause);
                return None;
            }
            Ok(transferred) => transferred,
        };

        tracing::debug!("transferred {:?} bytes", transferred);

        match transferred {
            0 => None,
            _ => Some(data),
        }
    }

    pub(super) fn receive(&self, mimetype: impl Into<String>) -> Option<Vec<u8>> {
        let mimetype: String = mimetype.into();
        if let Some(offer) = self.inner.devices.borrow().receive(&mimetype) {
            return self.initiate(offer);
        }

        None
    }
}

/// The system clipboard.
#[derive(Debug, Clone)]
pub struct Clipboard {
    inner: Manager,
}

impl From<&Manager> for Clipboard {
    fn from(m: &Manager) -> Self {
        Self { inner: m.clone() }
    }
}

impl Clipboard {
    const UTF8: &'static str = "text/plain;charset=utf-8";
    const TEXT: &'static str = "text/plain";
    const UTF8_STRING: &'static str = "UTF8_STRING";

    /// Put a string onto the system clipboard.
    pub fn put_string(&mut self, s: impl AsRef<str>) {
        let _s = s.as_ref().to_string();
        self.inner.inner.wdsobj.offer(Clipboard::UTF8.to_string());
    }

    /// Put multi-format data on the system clipboard.
    pub fn put_formats(&mut self, _formats: &[ClipboardFormat]) {
        tracing::warn!("clipboard copy not implemented");
    }

    /// Get a string from the system clipboard, if one is available.
    pub fn get_string(&self) -> Option<String> {
        [Clipboard::UTF8, Clipboard::TEXT, Clipboard::UTF8_STRING]
            .iter()
            .find_map(
                |mimetype| match std::str::from_utf8(&self.inner.receive(*mimetype)?) {
                    Ok(s) => Some(s.to_string()),
                    Err(cause) => {
                        tracing::error!("clipboard unable to retrieve utf8 content {:?}", cause);
                        None
                    }
                },
            )
    }

    /// Given a list of supported clipboard types, returns the supported type which has
    /// highest priority on the system clipboard, or `None` if no types are supported.
    pub fn preferred_format(&self, _formats: &[FormatId]) -> Option<FormatId> {
        tracing::warn!("clipboard preferred_format not implemented");
        None
    }

    /// Return data in a given format, if available.
    ///
    /// It is recommended that the `fmt` argument be a format returned by
    /// [`Clipboard::preferred_format`]
    pub fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        self.inner.receive(format)
    }

    pub fn available_type_names(&self) -> Vec<String> {
        tracing::warn!("clipboard available_type_names not implemented");
        Vec::new()
    }
}
