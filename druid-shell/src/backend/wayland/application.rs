// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(clippy::single_match)]

use super::{
    clipboard::Clipboard, error::Error, events::WaylandSource, keyboard, pointers, surfaces,
    window::WindowHandle,
};

use crate::{backend, kurbo, mouse, AppHandler, TimerToken};

use calloop;

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BinaryHeap},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::backend::shared::linux;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::{
    self as wl,
    protocol::{
        wl_compositor::WlCompositor,
        wl_output::{self, Subpixel, Transform, WlOutput},
        wl_pointer::WlPointer,
        wl_seat::{self, WlSeat},
        wl_shm::{self, WlShm},
        wl_surface::WlSurface,
    },
};
use wayland_cursor::CursorTheme;
use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols::xdg_shell::client::xdg_positioner::XdgPositioner;
use wayland_protocols::xdg_shell::client::xdg_surface;
use wayland_protocols::xdg_shell::client::xdg_wm_base::{self, XdgWmBase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timer(backend::shared::Timer<u64>);

impl Timer {
    pub(crate) fn new(id: u64, deadline: Instant) -> Self {
        Self(backend::shared::Timer::new(deadline, id))
    }

    pub(crate) fn id(self) -> u64 {
        self.0.data
    }

    pub(crate) fn deadline(&self) -> Instant {
        self.0.deadline()
    }

    pub fn token(&self) -> TimerToken {
        self.0.token()
    }
}

impl std::cmp::Ord for Timer {
    /// Ordering is so that earliest deadline sorts first
    // "Earliest deadline first" that a std::collections::BinaryHeap will have the earliest timer
    // at its head, which is just what is needed for timer management.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.deadline().cmp(&other.0.deadline()).reverse()
    }
}

impl std::cmp::PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct Application {
    pub(super) data: std::sync::Arc<ApplicationData>,
}

#[allow(dead_code)]
pub(crate) struct ApplicationData {
    pub(super) wl_server: wl::Display,
    pub(super) event_queue: Rc<RefCell<wl::EventQueue>>,
    // Wayland globals
    pub(super) globals: wl::GlobalManager,
    pub(super) xdg_base: wl::Main<XdgWmBase>,
    pub(super) zxdg_decoration_manager_v1: wl::Main<ZxdgDecorationManagerV1>,
    pub(super) zwlr_layershell_v1: wl::Main<ZwlrLayerShellV1>,
    pub(super) wl_compositor: wl::Main<WlCompositor>,
    pub(super) wl_shm: wl::Main<WlShm>,
    /// A map of wayland object IDs to outputs.
    ///
    /// Wayland will update this if the output change. Keep a record of the `Instant` you last
    /// observed a change, and use `Output::changed` to see if there are any newer changes.
    ///
    /// It's a BTreeMap so the ordering is consistent when enumerating outputs (not sure if this is
    /// necessary, but it negligable cost).
    pub(super) outputs: Rc<RefCell<BTreeMap<u32, Output>>>,
    pub(super) seats: Rc<RefCell<BTreeMap<u32, Rc<RefCell<Seat>>>>>,
    /// Handles to any surfaces that have been created.
    ///
    /// This is where the window data is owned. Other handles should be weak.
    // pub(super) surfaces: RefCell<im::OrdMap<u32, std::sync::Arc<surfaces::surface::Data>>>,

    /// Handles to any surfaces that have been created.
    pub(super) handles: RefCell<im::OrdMap<u64, WindowHandle>>,

    /// Available pixel formats
    pub(super) formats: RefCell<Vec<wl_shm::Format>>,
    /// Close flag
    pub(super) shutdown: Cell<bool>,
    /// The currently active surface, if any (by wayland object ID)
    pub(super) active_surface_id: RefCell<std::collections::VecDeque<u64>>,
    // Stuff for timers
    /// A calloop event source for timers. We always set it to fire at the next set timer, if any.
    pub(super) timer_handle: calloop::timer::TimerHandle<TimerToken>,
    /// We stuff this here until the event loop, then `take` it and use it.
    timer_source: RefCell<Option<calloop::timer::Timer<TimerToken>>>,
    /// Currently pending timers
    ///
    /// The extra data is the surface this timer is for.
    pub(super) timers: RefCell<BinaryHeap<Timer>>,

    pub(super) roundtrip_requested: RefCell<bool>,

    /// track if the display was flushed during the event loop.
    /// prevents double flushing unnecessarily.
    pub(super) display_flushed: RefCell<bool>,
    /// reference to the pointer events manager.
    pub(super) pointer: pointers::Pointer,
    /// reference to the keyboard events manager.
    keyboard: keyboard::Manager,
    // wakeup events when outputs are added/removed.
    outputs_removed: RefCell<Option<calloop::channel::Channel<Output>>>,
    outputs_added: RefCell<Option<calloop::channel::Channel<Output>>>,
}

impl Application {
    pub fn new() -> Result<Self, Error> {
        tracing::info!("wayland application initiated");
        // connect to the server. Internally an `Arc`, so cloning is cheap. Must be kept alive for
        // the duration of the app.
        let wl_server = wl::Display::connect_to_env()?;

        // create an event queue (required for receiving events from the server)
        let mut event_queue = wl_server.create_event_queue();

        // Tell wayland to use our event queue for creating new objects (applies recursively).
        let attached_server = (*wl_server).clone().attach(event_queue.token());

        // Global objects that can come and go (so we must handle them dynamically).
        //
        // They have to be behind a shared pointer because wayland may need to add or remove them
        // for the life of the application. Use weak rcs inside the callbacks to avoid leaking
        // memory.
        let outputs: Rc<RefCell<BTreeMap<u32, Output>>> = Rc::new(RefCell::new(BTreeMap::new()));
        let seats: Rc<RefCell<BTreeMap<u32, Rc<RefCell<Seat>>>>> =
            Rc::new(RefCell::new(BTreeMap::new()));
        // This object will create a container for the global wayland objects, and request that
        // it is populated by the server. Doesn't take ownership of the registry, we are
        // responsible for keeping it alive.
        let weak_outputs = Rc::downgrade(&outputs);
        let weak_seats = Rc::downgrade(&seats);

        let (outputsremovedtx, outputsremovedrx) = calloop::channel::channel::<Output>();
        let (outputsaddedtx, outputsaddedrx) = calloop::channel::channel::<Output>();

        let globals = wl::GlobalManager::new_with_cb(&attached_server, {
            move |event, registry, data| {
                tracing::debug!(
                    "global manager event received {:?}\n{:?}\n{:?}",
                    event,
                    registry,
                    data
                );
                match event {
                    wl::GlobalEvent::New {
                        id,
                        interface,
                        version,
                    } => {
                        if interface.as_str() == "wl_output" && version >= 3 {
                            let output = registry.bind::<WlOutput>(3, id);
                            let output = Output::new(id, output);
                            let oid = output.id();
                            let gid = output.gid;
                            let previous = weak_outputs
                                .upgrade()
                                .unwrap()
                                .borrow_mut()
                                .insert(oid, output.clone());
                            assert!(
                                previous.is_none(),
                                "internal: wayland should always use new IDs"
                            );
                            tracing::trace!("output added {:?} {:?}", gid, oid);
                            output.wl_output.quick_assign(with_cloned!(weak_outputs, gid, oid, outputsaddedtx; move |a, event, b| {
                                    tracing::trace!("output event {:?} {:?} {:?}", a, event, b);
                                    match weak_outputs.upgrade().unwrap().borrow_mut().get_mut(&oid) {
                                        Some(o) => o.process_event(event, &outputsaddedtx),
                                        None => tracing::warn!(
                                            "wayland sent an event for an output that doesn't exist global({:?}) proxy({:?}) {:?}",
                                            &gid,
                                            &oid,
                                            &event,
                                        ),
                                    }
                                }));
                        } else if interface.as_str() == "wl_seat" && version >= 7 {
                            let new_seat = registry.bind::<WlSeat>(7, id);
                            let prev_seat = weak_seats
                                .upgrade()
                                .unwrap()
                                .borrow_mut()
                                .insert(id, Rc::new(RefCell::new(Seat::new(new_seat))));
                            assert!(
                                prev_seat.is_none(),
                                "internal: wayland should always use new IDs"
                            );
                            // Defer setting up the pointer/keyboard event handling until we've
                            // finished constructing the `Application`. That way we can pass it as a
                            // parameter.
                        }
                    }
                    wl::GlobalEvent::Removed { id, interface }
                        if interface.as_str() == "wl_output" =>
                    {
                        let boutputs = weak_outputs.upgrade().unwrap();
                        let mut outputs = boutputs.borrow_mut();
                        let removed = outputs
                            .iter()
                            .find(|(_pid, o)| o.gid == id)
                            .map(|(pid, _)| pid.clone())
                            .and_then(|id| outputs.remove(&id));

                        let result = match removed {
                            None => return,
                            Some(removed) => outputsremovedtx.send(removed),
                        };

                        match result {
                            Ok(_) => tracing::debug!("outputs remaining {:?}...", outputs.len()),
                            Err(cause) => tracing::error!("failed to remove output {:?}", cause),
                        }
                    }
                    _ => {
                        tracing::debug!("unhandled global manager event received {:?}", event);
                    }
                }
            }
        });

        // do a round trip to make sure we have all the globals
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| unreachable!())
            .map_err(Error::fatal)?;

        let mut globals_list = globals.list();
        globals_list.sort_by(|(_, name1, version1), (_, name2, version2)| {
            name1.cmp(name2).then(version1.cmp(version2))
        });

        for (id, name, version) in globals_list.into_iter() {
            tracing::debug!("{:?}@{:?} - {:?}", name, version, id);
        }

        let xdg_base = globals
            .instantiate_exact::<XdgWmBase>(2)
            .map_err(|e| Error::global("xdg_wm_base", 2, e))?;
        let zxdg_decoration_manager_v1 = globals
            .instantiate_exact::<ZxdgDecorationManagerV1>(1)
            .map_err(|e| Error::global("zxdg_decoration_manager_v1", 1, e))?;
        let zwlr_layershell_v1 = globals
            .instantiate_exact::<ZwlrLayerShellV1>(1)
            .map_err(|e| Error::global("zwlr_layershell_v1", 1, e))?;
        let wl_compositor = globals
            .instantiate_exact::<WlCompositor>(4)
            .map_err(|e| Error::global("wl_compositor", 4, e))?;
        let wl_shm = globals
            .instantiate_exact::<WlShm>(1)
            .map_err(|e| Error::global("wl_shm", 1, e))?;

        // We do this to make sure wayland knows we're still responsive.
        //
        // NOTE: This means that clients mustn't hold up the event loop, or else wayland might kill
        // your app's connection. Move *everything* to another thread, including e.g. file i/o,
        // computation, network, ... This is good practice for all back-ends: it will improve
        // responsiveness.
        xdg_base.quick_assign(|xdg_base, event, d3| {
            tracing::info!("xdg_base events {:?} {:?} {:?}", xdg_base, event, d3);
            match event {
                xdg_wm_base::Event::Ping { serial } => xdg_base.pong(serial),
                _ => (),
            }
        });

        let timer_source = calloop::timer::Timer::new().unwrap();
        let timer_handle = timer_source.handle();

        // TODO the cursor theme size needs more refinement, it should probably be the size needed to
        // draw sharp cursors on the largest scaled monitor.
        let pointer = pointers::Pointer::new(
            CursorTheme::load(64, &wl_shm),
            wl_compositor.create_surface(),
        );

        // We need to have keyboard events set up for our seats before the next roundtrip.
        let app_data = std::sync::Arc::new(ApplicationData {
            wl_server,
            event_queue: Rc::new(RefCell::new(event_queue)),
            globals,
            xdg_base,
            zxdg_decoration_manager_v1,
            zwlr_layershell_v1,
            wl_compositor,
            wl_shm: wl_shm.clone(),
            outputs,
            seats,
            handles: RefCell::new(im::OrdMap::new()),
            formats: RefCell::new(vec![]),
            shutdown: Cell::new(false),
            active_surface_id: RefCell::new(std::collections::VecDeque::with_capacity(20)),
            timer_handle,
            timer_source: RefCell::new(Some(timer_source)),
            timers: RefCell::new(BinaryHeap::new()),
            display_flushed: RefCell::new(false),
            pointer,
            keyboard: keyboard::Manager::default(),
            roundtrip_requested: RefCell::new(false),
            outputs_added: RefCell::new(Some(outputsaddedrx)),
            outputs_removed: RefCell::new(Some(outputsremovedrx)),
        });

        // Collect the supported image formats.
        wl_shm.quick_assign(with_cloned!(app_data; move |d1, event, d3| {
            tracing::info!("shared memory events {:?} {:?} {:?}", d1, event, d3);
            match event {
                wl_shm::Event::Format { format } => app_data.formats.borrow_mut().push(format),
                _ => (), // ignore other messages
            }
        }));

        // Setup seat event listeners with our application
        for (id, seat) in app_data.seats.borrow().iter() {
            let id = *id; // move into closure.
            let wl_seat = seat.borrow().wl_seat.clone();
            wl_seat.quick_assign(with_cloned!(seat, app_data; move |d1, event, d3| {
                tracing::info!("seat events {:?} {:?} {:?}", d1, event, d3);
                let mut seat = seat.borrow_mut();
                match event {
                    wl_seat::Event::Capabilities { capabilities } => {
                        seat.capabilities = capabilities;
                        if capabilities.contains(wl_seat::Capability::Keyboard)
                            && seat.keyboard.is_none()
                        {
                            seat.keyboard = Some(app_data.keyboard.attach(app_data.clone(), id, seat.wl_seat.clone()));
                        }
                        if capabilities.contains(wl_seat::Capability::Pointer)
                            && seat.pointer.is_none()
                        {
                            let pointer = seat.wl_seat.get_pointer();
                            app_data.pointer.attach(pointer.detach());
                            pointer.quick_assign({
                                let app = app_data.clone();
                                move |pointer, event, _| {
                                    pointers::Pointer::consume(app.clone(), pointer.detach(), event);
                                }
                            });
                            seat.pointer = Some(pointer);
                        }
                        // Dont worry if they go away - we will just stop receiving events. If the
                        // capability comes back we will start getting events again.
                    }
                    wl_seat::Event::Name { name } => {
                        seat.name = name;
                    }
                    _ => tracing::info!("seat quick assign unknown event {:?}", event), // ignore future events
                }
            }));
        }

        // Let wayland finish setup before we allow the client to start creating windows etc.
        app_data.sync()?;

        Ok(Application { data: app_data })
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        tracing::info!("run initiated");
        // NOTE if we want to call this function more than once, we will need to put the timer
        // source back.
        let timer_source = self.data.timer_source.borrow_mut().take().unwrap();
        // flush pending events (otherwise anything we submitted since sync will never be sent)
        self.data.wl_server.flush().unwrap();
        // Use calloop so we can epoll both wayland events and others (e.g. timers)
        let mut eventloop = calloop::EventLoop::try_new().unwrap();
        let handle = eventloop.handle();

        let wayland_dispatcher = WaylandSource::new(self.data.clone()).into_dispatcher();

        handle.register_dispatcher(wayland_dispatcher).unwrap();
        handle
            .insert_source(self.data.outputs_added.borrow_mut().take().unwrap(), {
                move |evt, _ignored, appdata| match evt {
                    calloop::channel::Event::Closed => return,
                    calloop::channel::Event::Msg(output) => {
                        tracing::debug!("output added {:?} {:?}", output.gid, output.id());
                        for (_, win) in appdata.handles_iter() {
                            surfaces::Outputs::inserted(&win, &output);
                        }
                    }
                }
            })
            .unwrap();
        handle
            .insert_source(
                self.data.outputs_removed.borrow_mut().take().unwrap(),
                |evt, _ignored, appdata| match evt {
                    calloop::channel::Event::Closed => return,
                    calloop::channel::Event::Msg(output) => {
                        tracing::trace!("output removed {:?} {:?}", output.gid, output.id());
                        for (_, win) in appdata.handles_iter() {
                            surfaces::Outputs::removed(&win, &output);
                        }
                    }
                },
            )
            .unwrap();

        handle
            .insert_source(timer_source, move |token, _metadata, appdata| {
                tracing::trace!("timer source {:?}", token);
                appdata.handle_timer_event(token);
            })
            .unwrap();

        let signal = eventloop.get_signal();
        let handle = handle.clone();

        eventloop
            .run(
                Duration::from_millis(20),
                &mut self.data.clone(),
                move |appdata| {
                    if appdata.shutdown.get() {
                        tracing::debug!("shutting down");
                        signal.stop();
                    }

                    ApplicationData::idle_repaint(handle.clone());
                },
            )
            .unwrap();
    }

    pub fn quit(&self) {
        self.data.shutdown.set(true);
    }

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        linux::env::locale()
    }
}

impl surfaces::Compositor for ApplicationData {
    fn output(&self, id: &u32) -> Option<Output> {
        match self.outputs.borrow().get(id) {
            None => None,
            Some(o) => Some(o.clone()),
        }
    }

    fn create_surface(&self) -> wl::Main<WlSurface> {
        self.wl_compositor.create_surface()
    }

    fn shared_mem(&self) -> wl::Main<WlShm> {
        self.wl_shm.clone()
    }

    fn get_xdg_positioner(&self) -> wl::Main<XdgPositioner> {
        self.xdg_base.create_positioner()
    }

    fn get_xdg_surface(&self, s: &wl::Main<WlSurface>) -> wl::Main<xdg_surface::XdgSurface> {
        self.xdg_base.get_xdg_surface(s)
    }

    fn zxdg_decoration_manager_v1(&self) -> wl::Main<ZxdgDecorationManagerV1> {
        self.zxdg_decoration_manager_v1.clone()
    }

    fn zwlr_layershell_v1(&self) -> wl::Main<ZwlrLayerShellV1> {
        self.zwlr_layershell_v1.clone()
    }
}

impl ApplicationData {
    pub(crate) fn set_cursor(&self, cursor: &mouse::Cursor) {
        self.pointer.replace(&cursor);
    }

    /// Send all pending messages and process all received messages.
    ///
    /// Don't use this once the event loop has started.
    pub(crate) fn sync(&self) -> Result<(), Error> {
        self.event_queue
            .borrow_mut()
            .sync_roundtrip(&mut (), |evt, _, _| {
                panic!("unexpected wayland event: {:?}", evt)
            })
            .map_err(Error::fatal)?;
        Ok(())
    }

    fn current_window_id(&self) -> u64 {
        static DEFAULT: u64 = 0 as u64;
        self.active_surface_id
            .borrow()
            .get(0)
            .unwrap_or_else(|| &DEFAULT)
            .clone()
    }

    pub(super) fn initial_window_size(&self, defaults: kurbo::Size) -> kurbo::Size {
        // compute the initial window size.
        let initialwidth = if defaults.width == 0.0 {
            f64::INFINITY
        } else {
            defaults.width
        };
        let initialheight = if defaults.height == 0.0 {
            f64::INFINITY
        } else {
            defaults.height
        };
        return self.outputs.borrow().iter().fold(
            kurbo::Size::from((initialwidth, initialheight)),
            |computed, entry| match &entry.1.current_mode {
                None => computed,
                Some(mode) => kurbo::Size::new(
                    computed.width.min(mode.width.into()),
                    computed.height.min(mode.height.into()),
                ),
            },
        );
    }

    pub(super) fn acquire_current_window(&self) -> Option<WindowHandle> {
        match self.handles.borrow().get(&self.current_window_id()) {
            None => None,
            Some(w) => Some(w.clone()),
        }
    }

    pub(super) fn popup<'a>(&self, surface: &'a surfaces::popup::Surface) -> Result<(), Error> {
        match self.acquire_current_window() {
            None => return Err(Error::string("parent window does not exist")),
            Some(winhandle) => winhandle.popup(surface),
        }
    }

    fn handle_timer_event(&self, _token: TimerToken) {
        // Shouldn't be necessary.
        self.timer_handle.cancel_all_timeouts();
        // Don't borrow the timers in case the callbacks want to add more.
        // TODO make this in the stack (smallvec)
        let mut expired_timers = Vec::with_capacity(1);
        let mut timers = self.timers.borrow_mut();
        let now = Instant::now();
        while matches!(timers.peek(), Some(timer) if timer.deadline() < now) {
            // timer has passed
            expired_timers.push(timers.pop().unwrap());
        }
        drop(timers);
        for expired in expired_timers {
            let win = match self.handles.borrow().get(&expired.id()).cloned() {
                Some(s) => s,
                None => {
                    // NOTE this might be expected
                    log::warn!(
                        "received event for surface that doesn't exist any more {:?} {:?}",
                        expired,
                        expired.id()
                    );
                    continue;
                }
            };
            // re-entrancy
            win.data()
                .map(|data| data.handler.borrow_mut().timer(expired.token()));
        }

        for (_, win) in self.handles_iter() {
            win.data().map(|data| data.run_deferred_tasks());
        }

        // Get the deadline soonest and queue it.
        if let Some(timer) = self.timers.borrow().peek() {
            self.timer_handle
                .add_timeout(timer.deadline() - now, timer.token());
        }
        // Now flush so the events actually get sent (we don't do this automatically because we
        // aren't in a wayland callback.
        self.wl_server.flush().unwrap();
    }

    /// Shallow clones surfaces so we can modify it during iteration.
    fn handles_iter(&self) -> impl Iterator<Item = (u64, WindowHandle)> {
        self.handles.borrow().clone().into_iter()
    }

    fn idle_repaint<'a>(loophandle: calloop::LoopHandle<'a, std::sync::Arc<ApplicationData>>) {
        loophandle.insert_idle({
            move |appdata| {
                match appdata.acquire_current_window() {
                    Some(winhandle) => {
                        tracing::trace!("idle processing initiated");

                        winhandle.request_anim_frame();
                        winhandle.run_idle();
                        // if we already flushed this cycle don't flush again.
                        if *appdata.display_flushed.borrow() {
                            tracing::trace!("idle repaint flushing display initiated");
                            if let Err(cause) = appdata.event_queue.borrow().display().flush() {
                                tracing::warn!("unable to flush display: {:?}", cause);
                            }
                        }
                        tracing::trace!("idle processing completed");
                    }
                    None => tracing::error!(
                        "unable to acquire current window, skipping idle processing"
                    ),
                };
            }
        });
    }
}

impl From<Application> for surfaces::CompositorHandle {
    fn from(app: Application) -> surfaces::CompositorHandle {
        surfaces::CompositorHandle::from(app.data)
    }
}

impl From<std::sync::Arc<ApplicationData>> for surfaces::CompositorHandle {
    fn from(data: std::sync::Arc<ApplicationData>) -> surfaces::CompositorHandle {
        surfaces::CompositorHandle::direct(
            std::sync::Arc::downgrade(&data) as std::sync::Weak<dyn surfaces::Compositor>
        )
    }
}

#[derive(Debug, Clone)]
pub struct Output {
    wl_output: wl::Main<WlOutput>,
    wl_proxy: wl::Proxy<WlOutput>,
    /// global id of surface.
    pub gid: u32,
    pub x: i32,
    pub y: i32,
    pub physical_width: i32,
    pub physical_height: i32,
    pub subpixel: Subpixel,
    pub make: String,
    pub model: String,
    pub transform: Transform,
    pub scale: i32,
    pub current_mode: Option<Mode>,
    pub preferred_mode: Option<Mode>,
    /// Whether we have received some update events but not the `done` event.
    update_in_progress: bool,
    /// Lets us work out if things have changed since we last observed the output.
    last_update: Instant,
}

#[allow(unused)]
impl Output {
    // All the stuff before `current_mode` will be filled out immediately after creation, so these
    // dummy values will never be observed.
    fn new(id: u32, wl_output: wl::Main<WlOutput>) -> Self {
        Output {
            wl_output: wl_output.clone(),
            wl_proxy: wl::Proxy::from(wl_output.detach()),
            gid: id,
            x: 0,
            y: 0,
            physical_width: 0,
            physical_height: 0,
            subpixel: Subpixel::Unknown,
            make: "".into(),
            model: "".into(),
            transform: Transform::Normal,
            current_mode: None,
            preferred_mode: None,
            scale: 1, // the spec says if there is no scale event, assume 1.
            update_in_progress: true,
            last_update: Instant::now(),
        }
    }

    /// Get the wayland object ID for this output. This is how we key outputs in our global
    /// registry.
    pub fn id(&self) -> u32 {
        self.wl_proxy.id()
    }

    /// Incorporate update data from the server for this output.
    fn process_event(&mut self, evt: wl_output::Event, tx: &calloop::channel::Sender<Self>) {
        tracing::trace!("processing wayland output event {:?}", evt);
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
                self.x = x;
                self.y = y;
                self.subpixel = subpixel;
                self.make = make;
                self.model = model;
                self.transform = transform;
                self.update_in_progress = true;

                match transform {
                    wl_output::Transform::Flipped270 | wl_output::Transform::_270 => {
                        self.physical_width = physical_height;
                        self.physical_height = physical_width;
                    }
                    _ => {
                        self.physical_width = physical_width;
                        self.physical_height = physical_height;
                    }
                }
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                if flags.contains(wl_output::Mode::Current) {
                    self.current_mode = Some(Mode {
                        width,
                        height,
                        refresh,
                    });
                }
                if flags.contains(wl_output::Mode::Preferred) {
                    self.preferred_mode = Some(Mode {
                        width,
                        height,
                        refresh,
                    });
                }
                self.update_in_progress = true;
            }
            wl_output::Event::Done => {
                self.update_in_progress = false;
                self.last_update = Instant::now();
                if let Err(cause) = tx.send(self.clone()) {
                    tracing::error!("unable to add output {:?} {:?}", self.gid, self.id());
                }
            }
            wl_output::Event::Scale { factor } => {
                self.scale = factor;
                self.update_in_progress = true;
            }
            _ => tracing::warn!("unknown output event {:?}", evt), // ignore possible future events
        }
    }

    /// Whether the output has changed since `since`.
    ///
    /// Will return `false` if an update is in progress, as updates should be handled atomically.
    fn changed(&self, since: Instant) -> bool {
        !self.update_in_progress && since < self.last_update
    }
}

#[derive(Debug, Clone)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
}

#[derive(Debug, Clone)]
pub struct Seat {
    wl_seat: wl::Main<WlSeat>,
    name: String,
    capabilities: wl_seat::Capability,
    keyboard: Option<wl::Main<WlKeyboard>>,
    pointer: Option<wl::Main<WlPointer>>,
}

impl Seat {
    fn new(wl_seat: wl::Main<WlSeat>) -> Self {
        Self {
            wl_seat,
            name: "".into(),
            capabilities: wl_seat::Capability::empty(),
            keyboard: None,
            pointer: None,
        }
    }
}
