// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactions with the system pasteboard on X11.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;
use std::time::{Duration, Instant};

use x11rb::connection::{Connection, RequestConnection};
use x11rb::errors::{ConnectionError, ReplyError, ReplyOrIdError};
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ChangeWindowAttributesAux, ConnectionExt, EventMask, GetPropertyReply,
    GetPropertyType, PropMode, Property, PropertyNotifyEvent, SelectionClearEvent,
    SelectionNotifyEvent, SelectionRequestEvent, Timestamp, Window, WindowClass,
    SELECTION_NOTIFY_EVENT,
};
use x11rb::protocol::Event;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::xcb_ffi::XCBConnection;

use super::application::AppAtoms;
use crate::clipboard::{ClipboardFormat, FormatId};
use tracing::{debug, error, warn};

// We can pick an arbitrary atom that is used for the transfer. This is our pick.
const TRANSFER_ATOM: AtomEnum = AtomEnum::CUT_BUFFE_R4;

const STRING_TARGETS: [&str; 5] = [
    "UTF8_STRING",
    "TEXT",
    "STRING",
    "text/plain;charset=utf-8",
    "text/plain",
];

#[derive(Debug, Clone)]
pub struct Clipboard(Rc<RefCell<ClipboardState>>);

impl Clipboard {
    pub(crate) fn new(
        connection: Rc<XCBConnection>,
        screen_num: usize,
        atoms: Rc<AppAtoms>,
        selection_name: Atom,
        event_queue: Rc<RefCell<VecDeque<Event>>>,
        timestamp: Rc<Cell<Timestamp>>,
    ) -> Self {
        Self(Rc::new(RefCell::new(ClipboardState::new(
            connection,
            screen_num,
            atoms,
            selection_name,
            event_queue,
            timestamp,
        ))))
    }

    pub(crate) fn handle_clear(&self, event: SelectionClearEvent) -> Result<(), ConnectionError> {
        self.0.borrow_mut().handle_clear(event)
    }

    pub(crate) fn handle_request(
        &self,
        event: &SelectionRequestEvent,
    ) -> Result<(), ReplyOrIdError> {
        self.0.borrow_mut().handle_request(event)
    }

    pub(crate) fn handle_property_notify(
        &self,
        event: PropertyNotifyEvent,
    ) -> Result<(), ReplyOrIdError> {
        self.0.borrow_mut().handle_property_notify(event)
    }

    pub fn put_string(&mut self, s: impl AsRef<str>) {
        let bytes = s.as_ref().as_bytes();
        let formats = STRING_TARGETS
            .iter()
            .map(|format| ClipboardFormat::new(format, bytes))
            .collect::<Vec<_>>();
        self.put_formats(&formats);
    }

    pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
        if let Err(err) = self.0.borrow_mut().put_formats(formats) {
            error!("Error in Clipboard::put_formats: {:?}", err);
        }
    }

    pub fn get_string(&self) -> Option<String> {
        self.0.borrow().get_string()
    }

    pub fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        self.0.borrow().preferred_format(formats)
    }

    pub fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        self.0.borrow().get_format(format)
    }

    pub fn available_type_names(&self) -> Vec<String> {
        self.0.borrow().available_type_names()
    }
}

#[derive(Debug)]
struct ClipboardState {
    connection: Rc<XCBConnection>,
    screen_num: usize,
    atoms: Rc<AppAtoms>,
    selection_name: Atom,
    event_queue: Rc<RefCell<VecDeque<Event>>>,
    timestamp: Rc<Cell<Timestamp>>,
    contents: Option<ClipboardContents>,
    incremental: Vec<IncrementalTransfer>,
}

impl ClipboardState {
    fn new(
        connection: Rc<XCBConnection>,
        screen_num: usize,
        atoms: Rc<AppAtoms>,
        selection_name: Atom,
        event_queue: Rc<RefCell<VecDeque<Event>>>,
        timestamp: Rc<Cell<Timestamp>>,
    ) -> Self {
        Self {
            connection,
            screen_num,
            atoms,
            selection_name,
            event_queue,
            timestamp,
            contents: None,
            incremental: Vec::new(),
        }
    }

    fn put_formats(&mut self, formats: &[ClipboardFormat]) -> Result<(), ReplyOrIdError> {
        let conn = &*self.connection;

        // Create a window for selection ownership and save the necessary state
        let contents = ClipboardContents::new(conn, self.screen_num, formats)?;

        // Become selection owner of our selection
        conn.set_selection_owner(
            contents.owner_window,
            self.selection_name,
            self.timestamp.get(),
        )?;

        // Check if we really are the selection owner; this might e.g. fail if our timestamp is too
        // old and some other program became the selection owner with a newer timestamp
        let owner = conn.get_selection_owner(self.selection_name)?.reply()?;
        if owner.owner == contents.owner_window {
            // We are the new selection owner! Remember our contents for later.
            debug!("put_formats(): became selection owner");
            if let Some(mut old_owner) = std::mem::replace(&mut self.contents, Some(contents)) {
                // We already where the owner before. Destroy the old contents.
                old_owner.destroy(conn)?;
            }
        } else {
            debug!("put_formats(): failed to become selection owner");
        }

        Ok(())
    }

    fn get_string(&self) -> Option<String> {
        STRING_TARGETS.iter().find_map(|target| {
            self.get_format(target)
                .and_then(|data| String::from_utf8(data).ok())
        })
    }

    fn preferred_format(&self, formats: &[FormatId]) -> Option<FormatId> {
        let available = self.available_type_names();
        formats
            .iter()
            .find(|f1| available.iter().any(|f2| *f1 == f2))
            .copied()
    }

    fn get_format(&self, format: FormatId) -> Option<Vec<u8>> {
        if let Some(contents) = self.contents.as_ref() {
            // We are the selection owner and can directly return the result
            contents
                .data
                .iter()
                .find(|(_, fmt, _)| fmt == format)
                .map(|(_, _, data)| data.to_vec())
        } else {
            self.do_transfer(format, |prop| prop.value)
        }
    }

    #[allow(clippy::needless_collect)]
    fn available_type_names(&self) -> Vec<String> {
        if let Some(contents) = self.contents.as_ref() {
            // We are the selection owner and can directly return the result
            return contents
                .data
                .iter()
                .map(|(_, format, _)| format.to_string())
                .collect();
        }
        let requests = self
            .do_transfer("TARGETS", |prop| {
                prop.value32()
                    .map(|iter| iter.collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
            .into_iter()
            .filter_map(|atom| self.connection.get_atom_name(atom).ok())
            .collect::<Vec<_>>();
        // We first send all requests above and then fetch the replies with only one round-trip to
        // the X11 server. Hence, the collect() above is not unnecessary!
        requests
            .into_iter()
            .filter_map(|req| req.reply().ok())
            .filter_map(|reply| String::from_utf8(reply.name).ok())
            .collect()
    }

    fn do_transfer<R, F>(&self, format: FormatId, converter: F) -> Option<Vec<R>>
    where
        R: Clone,
        F: FnMut(GetPropertyReply) -> Vec<R>,
    {
        match self.do_transfer_impl(format, converter) {
            Ok(result) => result,
            Err(error) => {
                warn!("Error in Clipboard::do_transfer: {:?}", error);
                None
            }
        }
    }

    fn do_transfer_impl<R, F>(
        &self,
        format: FormatId,
        mut converter: F,
    ) -> Result<Option<Vec<R>>, ReplyOrIdError>
    where
        R: Clone,
        F: FnMut(GetPropertyReply) -> Vec<R>,
    {
        debug!("Getting clipboard contents in format {}", format);

        let deadline = Instant::now() + Duration::from_secs(5);

        let conn = &*self.connection;
        let format_atom = conn.intern_atom(false, format.as_bytes())?.reply()?.atom;

        // Create a window for the transfer
        let window = WindowContainer::new(conn, self.screen_num)?;

        conn.convert_selection(
            window.window,
            self.selection_name,
            format_atom,
            TRANSFER_ATOM,
            self.timestamp.get(),
        )?;

        // Now wait for the selection notify event
        conn.flush()?;
        let notify = loop {
            match wait_for_event_with_deadline(conn, deadline)? {
                Event::SelectionNotify(notify) if notify.requestor == window.window => {
                    break notify
                }
                Event::SelectionRequest(request) if request.requestor == window.window => {
                    // The callers should catch this situation before and not even call us
                    // do_transfer()
                    error!("BUG! We are doing a selection transfer while we are the selection owner. This will hang!");
                }
                event => self.event_queue.borrow_mut().push_back(event),
            }
        };

        if notify.property == x11rb::NONE {
            // Selection is empty
            debug!("Selection transfer was rejected");
            return Ok(None);
        }

        conn.change_window_attributes(
            window.window,
            &ChangeWindowAttributesAux::default().event_mask(EventMask::PROPERTY_CHANGE),
        )?;

        let property = conn
            .get_property(
                true,
                window.window,
                TRANSFER_ATOM,
                GetPropertyType::ANY,
                0,
                u32::MAX,
            )?
            .reply()?;

        if property.type_ != self.atoms.INCR {
            debug!("Got selection contents directly");
            return Ok(Some(converter(property)));
        }

        // The above GetProperty with delete=true indicated that the INCR transfer starts
        // now, wait for the property notifies
        debug!("Doing an INCR transfer for the selection");
        conn.flush()?;
        let mut value = Vec::new();
        loop {
            match wait_for_event_with_deadline(conn, deadline)? {
                Event::PropertyNotify(notify)
                    if (notify.window, notify.state) == (window.window, Property::NEW_VALUE) =>
                {
                    let property = conn
                        .get_property(
                            true,
                            window.window,
                            TRANSFER_ATOM,
                            GetPropertyType::ANY,
                            0,
                            u32::MAX,
                        )?
                        .reply()?;
                    if property.value.is_empty() {
                        debug!("INCR transfer finished");
                        return Ok(Some(value));
                    } else {
                        value.extend_from_slice(&converter(property));
                    }
                }
                event => self.event_queue.borrow_mut().push_back(event),
            }
        }
    }

    fn handle_clear(&mut self, event: SelectionClearEvent) -> Result<(), ConnectionError> {
        if event.selection != self.selection_name {
            // This event is meant for another Clipboard instance
            return Ok(());
        }

        let window = self.contents.as_ref().map(|c| c.owner_window);
        if Some(event.owner) == window {
            // We lost ownership of the selection, clean up
            if let Some(mut contents) = self.contents.take() {
                contents.destroy(&self.connection)?;
            }
        }
        Ok(())
    }

    fn handle_request(&mut self, event: &SelectionRequestEvent) -> Result<(), ReplyOrIdError> {
        if event.selection != self.selection_name {
            // This request is meant for another Clipboard instance
            return Ok(());
        }

        let conn = &*self.connection;
        let contents = match &self.contents {
            Some(contents) if contents.owner_window == event.owner => contents,
            _ => {
                // We do not know what to do with this transfer
                debug!("Got non-matching selection request event");
                reject_transfer(conn, event)?;
                return Ok(());
            }
        };

        // TODO: ICCCM has TIMESTAMP as a required target (but no one uses it...?)
        if event.target == self.atoms.TARGETS {
            // TARGETS is a special case: reply is list of u32
            let mut atoms = contents
                .data
                .iter()
                .map(|(atom, _, _)| *atom)
                .collect::<Vec<_>>();
            atoms.push(self.atoms.TARGETS);
            conn.change_property32(
                PropMode::REPLACE,
                event.requestor,
                event.property,
                AtomEnum::ATOM,
                &atoms,
            )?;
        } else {
            // Find the request target
            let content = contents
                .data
                .iter()
                .find(|(atom, _, _)| *atom == event.target);
            match content {
                None => {
                    reject_transfer(conn, event)?;
                    return Ok(());
                }
                Some((atom, _, data)) => {
                    if data.len() > maximum_property_length(conn) {
                        // We need to do an INCR transfer.
                        debug!("Starting new INCR transfer");
                        let transfer =
                            IncrementalTransfer::new(conn, event, Rc::clone(data), self.atoms.INCR);
                        match transfer {
                            Ok(transfer) => self.incremental.push(transfer),
                            Err(err) => {
                                reject_transfer(conn, event)?;
                                return Err(err.into());
                            }
                        }
                    } else {
                        // We can provide the data directly
                        conn.change_property8(
                            PropMode::REPLACE,
                            event.requestor,
                            event.property,
                            *atom,
                            data,
                        )?;
                    }
                }
            }
        }

        // Inform the requestor that we sent the data
        debug!("Replying to selection request event");
        let event = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            requestor: event.requestor,
            selection: event.selection,
            target: event.target,
            property: event.property,
            time: event.time,
        };
        conn.send_event(false, event.requestor, EventMask::NO_EVENT, event)?;

        Ok(())
    }

    fn handle_property_notify(&mut self, event: PropertyNotifyEvent) -> Result<(), ReplyOrIdError> {
        fn matches(transfer: &IncrementalTransfer, event: PropertyNotifyEvent) -> bool {
            transfer.requestor == event.window && transfer.property == event.atom
        }

        if event.state != Property::DELETE {
            return Ok(());
        }
        // Deleting the target property indicates that an INCR transfer should continue. Find that
        // transfer
        if let Some(transfer) = self
            .incremental
            .iter_mut()
            .find(|transfer| matches(transfer, event))
        {
            let done = transfer.continue_incremental(&self.connection)?;
            if done {
                debug!("INCR transfer finished");
                // Remove the transfer
                self.incremental
                    .retain(|transfer| !matches(transfer, event));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ClipboardContents {
    owner_window: Window,
    data: Vec<(Atom, String, Rc<[u8]>)>,
}

impl ClipboardContents {
    fn new(
        conn: &XCBConnection,
        screen_num: usize,
        formats: &[ClipboardFormat],
    ) -> Result<Self, ReplyOrIdError> {
        // Send InternAtom requests for all formats
        let data = formats
            .iter()
            .map(|format| {
                conn.intern_atom(false, format.identifier.as_bytes())
                    .map(|cookie| (cookie, format))
            })
            .collect::<Result<Vec<_>, ConnectionError>>()?;
        // Get the replies for all InternAtom requests
        let data = data
            .into_iter()
            .map(|(cookie, format)| {
                cookie.reply().map(|reply| {
                    (
                        reply.atom,
                        format.identifier.to_string(),
                        format.data[..].into(),
                    )
                })
            })
            .collect::<Result<Vec<_>, ReplyError>>()?;

        let owner_window = conn.generate_id()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            owner_window,
            conn.setup().roots[screen_num].root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &Default::default(),
        )?;
        Ok(Self { owner_window, data })
    }

    fn destroy(&mut self, conn: &XCBConnection) -> Result<(), ConnectionError> {
        conn.destroy_window(std::mem::replace(&mut self.owner_window, x11rb::NONE))?;
        Ok(())
    }
}

#[derive(Debug)]
struct IncrementalTransfer {
    requestor: Window,
    target: Atom,
    property: Atom,
    data: Rc<[u8]>,
    data_offset: usize,
}

impl IncrementalTransfer {
    fn new(
        conn: &XCBConnection,
        event: &SelectionRequestEvent,
        data: Rc<[u8]>,
        incr: Atom,
    ) -> Result<Self, ConnectionError> {
        // We need PropertyChange events on the window
        conn.change_window_attributes(
            event.requestor,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?;
        // Indicate that we are doing an INCR transfer
        let length = u32::try_from(data.len()).unwrap_or(u32::MAX);
        conn.change_property32(
            PropMode::REPLACE,
            event.requestor,
            event.property,
            incr,
            &[length],
        )?;
        Ok(Self {
            requestor: event.requestor,
            target: event.target,
            property: event.property,
            data,
            data_offset: 0,
        })
    }

    /// Continue an incremental transfer, returning true if the transfer is finished
    fn continue_incremental(&mut self, conn: &XCBConnection) -> Result<bool, ConnectionError> {
        let remaining = &self.data[self.data_offset..];
        let next_length = remaining.len().min(maximum_property_length(conn));
        conn.change_property8(
            PropMode::REPLACE,
            self.requestor,
            self.property,
            self.target,
            &remaining[..next_length],
        )?;
        self.data_offset += next_length;
        Ok(remaining.is_empty())
    }
}

struct WindowContainer<'a> {
    window: u32,
    conn: &'a XCBConnection,
}

impl<'a> WindowContainer<'a> {
    fn new(conn: &'a XCBConnection, screen_num: usize) -> Result<Self, ReplyOrIdError> {
        let window = conn.generate_id()?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            conn.setup().roots[screen_num].root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &Default::default(),
        )?;
        Ok(WindowContainer { window, conn })
    }
}

impl Drop for WindowContainer<'_> {
    fn drop(&mut self) {
        let _ = self.conn.destroy_window(self.window);
    }
}

fn maximum_property_length(connection: &XCBConnection) -> usize {
    let change_property_header_size = 24;
    // Apply an arbitrary limit to the property size to not stress the server too much
    let max_request_length = connection
        .maximum_request_bytes()
        .min(usize::from(u16::MAX));
    max_request_length - change_property_header_size
}

fn reject_transfer(
    conn: &XCBConnection,
    event: &SelectionRequestEvent,
) -> Result<(), ConnectionError> {
    let event = SelectionNotifyEvent {
        response_type: SELECTION_NOTIFY_EVENT,
        sequence: 0,
        requestor: event.requestor,
        selection: event.selection,
        target: event.target,
        property: x11rb::NONE,
        time: event.time,
    };
    conn.send_event(false, event.requestor, EventMask::NO_EVENT, event)?;
    Ok(())
}

/// Wait for an X11 event or return a timeout error if the given deadline is in the past.
fn wait_for_event_with_deadline(
    conn: &XCBConnection,
    deadline: Instant,
) -> Result<Event, ConnectionError> {
    use nix::poll::{poll, PollFd, PollFlags};
    use std::os::raw::c_int;
    use std::os::unix::io::AsRawFd;

    loop {
        // Is there already an event?
        if let Some(event) = conn.poll_for_event()? {
            return Ok(event);
        }

        // Are we past the deadline?
        let now = Instant::now();
        if deadline <= now {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Timeout while waiting for selection owner to reply",
            )
            .into());
        }

        // Use poll() to wait for the socket to become readable.
        let mut poll_fds = [PollFd::new(conn.as_raw_fd(), PollFlags::POLLIN)];
        let poll_timeout = c_int::try_from(deadline.duration_since(now).as_millis())
            .unwrap_or(c_int::MAX - 1)
            // The above rounds down, but we don't want to wake up to early, so add one
            .saturating_add(1);

        // Wait for the socket to be readable via poll() and try again
        match poll(&mut poll_fds, poll_timeout) {
            Ok(_) => {}
            Err(nix::errno::Errno::EINTR) => {}
            Err(e) => return Err(std::io::Error::from(e).into()),
        }
    }
}
