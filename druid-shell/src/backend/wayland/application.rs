// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::single_match)]

use super::{
    clipboard, display, error::Error, events::WaylandSource, keyboard, outputs, pointers, surfaces,
    window::WindowHandle,
};

use crate::{backend, mouse, AppHandler, TimerToken};

use calloop;

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BinaryHeap},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::backend::shared::linux;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_registry;
use wayland_client::{
    self as wl,
    protocol::{
        wl_compositor::WlCompositor,
        wl_pointer::WlPointer,
        wl_region::WlRegion,
        wl_seat::{self, WlSeat},
        wl_shm::{self, WlShm},
        wl_surface::WlSurface,
    },
};
use wayland_cursor::CursorTheme;
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols::xdg_shell::client::xdg_positioner::XdgPositioner;
use wayland_protocols::xdg_shell::client::xdg_surface;

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
    pub(super) data: std::sync::Arc<Data>,
}

#[allow(dead_code)]
pub(crate) struct Data {
    pub(super) wayland: std::rc::Rc<display::Environment>,
    pub(super) zwlr_layershell_v1: Option<wl::Main<ZwlrLayerShellV1>>,
    pub(super) wl_compositor: wl::Main<WlCompositor>,
    pub(super) wl_shm: wl::Main<WlShm>,
    /// A map of wayland object IDs to outputs.
    ///
    /// Wayland will update this if the output change. Keep a record of the `Instant` you last
    /// observed a change, and use `Output::changed` to see if there are any newer changes.
    ///
    /// It's a BTreeMap so the ordering is consistent when enumerating outputs (not sure if this is
    /// necessary, but it negligible cost).
    pub(super) outputs: Rc<RefCell<BTreeMap<u32, outputs::Meta>>>,
    pub(super) seats: Rc<RefCell<BTreeMap<u32, Rc<RefCell<Seat>>>>>,

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
    clipboard: clipboard::Manager,
    // wakeup events when outputs are added/removed.
    outputsqueue: RefCell<Option<calloop::channel::Channel<outputs::Event>>>,
}

impl Application {
    pub fn new() -> Result<Self, Error> {
        tracing::info!("wayland application initiated");

        // Global objects that can come and go (so we must handle them dynamically).
        //
        // They have to be behind a shared pointer because wayland may need to add or remove them
        // for the life of the application. Use weak rcs inside the callbacks to avoid leaking
        // memory.
        let dispatcher = display::Dispatcher::default();
        let outputqueue = outputs::auto(&dispatcher)?;

        let seats: Rc<RefCell<BTreeMap<u32, Rc<RefCell<Seat>>>>> =
            Rc::new(RefCell::new(BTreeMap::new()));
        // This object will create a container for the global wayland objects, and request that
        // it is populated by the server. Doesn't take ownership of the registry, we are
        // responsible for keeping it alive.
        let weak_seats = Rc::downgrade(&seats);

        display::GlobalEventDispatch::subscribe(
            &dispatcher,
            move |event: &'_ wl::GlobalEvent,
                  registry: &'_ wl::Attached<wl_registry::WlRegistry>,
                  _ctx: &'_ wl::DispatchData| {
                match event {
                    wl::GlobalEvent::New {
                        id,
                        interface,
                        version,
                    } => {
                        let id = *id;
                        let version = *version;

                        if interface.as_str() != "wl_seat" {
                            return;
                        }

                        tracing::debug!("seat detected {:?} {:?} {:?}", interface, id, version);

                        // 7 is the max version supported by wayland-rs 0.29.5
                        let version = version.min(7);
                        let new_seat = registry.bind::<WlSeat>(version, id);
                        let prev_seat = weak_seats
                            .upgrade()
                            .unwrap()
                            .borrow_mut()
                            .insert(id, Rc::new(RefCell::new(Seat::new(new_seat))));
                        assert!(
                            prev_seat.is_none(),
                            "internal: wayland should always use new IDs"
                        );

                        // TODO: This code handles only app startup, but seats can come and go on the fly,
                        // so we have to handle that in the future

                        // Defer setting up the pointer/keyboard event handling until we've
                        // finished constructing the `Application`. That way we can pass it as a
                        // parameter.
                    }
                    wl::GlobalEvent::Removed { .. } => {
                        // nothing to do.
                    }
                };
            },
        );

        let env = display::new(dispatcher)?;
        display::print(&env.registry);

        let zwlr_layershell_v1 = env
            .registry
            .instantiate_exact::<ZwlrLayerShellV1>(1)
            .map_or_else(
                |e| {
                    tracing::info!("unable to instantiate layershell {:?}", e);
                    None
                },
                Some,
            );

        let wl_compositor = env
            .registry
            .instantiate_range::<WlCompositor>(1, 5)
            .map_err(|e| Error::global("wl_compositor", 1, e))?;
        let wl_shm = env
            .registry
            .instantiate_exact::<WlShm>(1)
            .map_err(|e| Error::global("wl_shm", 1, e))?;

        let timer_source = calloop::timer::Timer::new().unwrap();
        let timer_handle = timer_source.handle();

        // TODO the cursor theme size needs more refinement, it should probably be the size needed to
        // draw sharp cursors on the largest scaled monitor.
        let pointer = pointers::Pointer::new(
            CursorTheme::load(64, &wl_shm),
            wl_compositor.create_surface(),
        );

        // We need to have keyboard events set up for our seats before the next roundtrip.
        let appdata = std::sync::Arc::new(Data {
            zwlr_layershell_v1,
            wl_compositor,
            wl_shm: wl_shm.clone(),
            outputs: Rc::new(RefCell::new(BTreeMap::new())),
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
            clipboard: clipboard::Manager::new(&env.display, &env.registry)?,
            roundtrip_requested: RefCell::new(false),
            outputsqueue: RefCell::new(Some(outputqueue)),
            wayland: std::rc::Rc::new(env),
        });

        // Collect the supported image formats.
        wl_shm.quick_assign(with_cloned!(appdata; move |d1, event, d3| {
            tracing::debug!("shared memory events {:?} {:?} {:?}", d1, event, d3);
            match event {
                wl_shm::Event::Format { format } => appdata.formats.borrow_mut().push(format),
                _ => (), // ignore other messages
            }
        }));

        // Setup seat event listeners with our application
        for (id, seat) in appdata.seats.borrow().iter() {
            let id = *id; // move into closure.
            let wl_seat = seat.borrow().wl_seat.clone();
            wl_seat.quick_assign(with_cloned!(seat, appdata; move |d1, event, d3| {
                tracing::debug!("seat events {:?} {:?} {:?}", d1, event, d3);
                let mut seat = seat.borrow_mut();
                appdata.clipboard.attach(&mut seat);
                match event {
                    wl_seat::Event::Capabilities { capabilities } => {
                        seat.capabilities = capabilities;
                        if capabilities.contains(wl_seat::Capability::Keyboard)
                            && seat.keyboard.is_none()
                        {
                            seat.keyboard = Some(appdata.keyboard.attach(id, seat.wl_seat.clone()));
                        }
                        if capabilities.contains(wl_seat::Capability::Pointer)
                            && seat.pointer.is_none()
                        {
                            let pointer = seat.wl_seat.get_pointer();
                            appdata.pointer.attach(pointer.detach());
                            pointer.quick_assign({
                                let app = appdata.clone();
                                move |pointer, event, _| {
                                    pointers::Pointer::consume(app.clone(), pointer.detach(), event);
                                }
                            });
                            seat.pointer = Some(pointer);
                        }

                        // TODO: We should react to capability removal, 
                        // "if a seat regains the pointer capability 
                        // and a client has a previously obtained wl_pointer object 
                        // of version 4 or less, that object may start sending pointer events again. 
                        // This behavior is considered a misinterpretation of the intended behavior 
                        // and must not be relied upon by the client", 
                        // versions 5 up guarantee that events will not be sent for sure
                    }
                    wl_seat::Event::Name { name } => {
                        seat.name = name;
                    }
                    _ => tracing::info!("seat quick assign unknown event {:?}", event), // ignore future events
                }
            }));
        }

        // Let wayland finish setup before we allow the client to start creating windows etc.
        appdata.sync()?;

        Ok(Application { data: appdata })
    }

    pub fn run(mut self, _handler: Option<Box<dyn AppHandler>>) {
        tracing::info!("wayland event loop initiated");
        // NOTE if we want to call this function more than once, we will need to put the timer
        // source back.
        let timer_source = self.data.timer_source.borrow_mut().take().unwrap();
        // flush pending events (otherwise anything we submitted since sync will never be sent)
        self.data.wayland.display.flush().unwrap();

        // Use calloop so we can epoll both wayland events and others (e.g. timers)
        let mut eventloop = calloop::EventLoop::try_new().unwrap();
        let handle = eventloop.handle();

        let wayland_dispatcher = WaylandSource::new(self.data.clone()).into_dispatcher();

        self.data.keyboard.events(&handle);

        handle.register_dispatcher(wayland_dispatcher).unwrap();
        handle
            .insert_source(self.data.outputsqueue.take().unwrap(), {
                move |evt, _ignored, appdata| match evt {
                    calloop::channel::Event::Closed => {}
                    calloop::channel::Event::Msg(output) => match output {
                        outputs::Event::Located(output) => {
                            tracing::debug!("output added {:?} {:?}", output.gid, output.id());
                            appdata
                                .outputs
                                .borrow_mut()
                                .insert(output.id(), output.clone());
                            for (_, win) in appdata.handles_iter() {
                                surfaces::Outputs::inserted(&win, &output);
                            }
                        }
                        outputs::Event::Removed(output) => {
                            tracing::debug!("output removed {:?} {:?}", output.gid, output.id());
                            appdata.outputs.borrow_mut().remove(&output.id());
                            for (_, win) in appdata.handles_iter() {
                                surfaces::Outputs::removed(&win, &output);
                            }
                        }
                    },
                }
            })
            .unwrap();

        handle
            .insert_source(timer_source, move |token, _metadata, appdata| {
                tracing::trace!("timer source {:?}", token);
                appdata.handle_timer_event(token);
            })
            .unwrap();

        let signal = eventloop.get_signal();
        let handle = handle.clone();

        let res = eventloop.run(Duration::from_millis(20), &mut self.data, move |appdata| {
            if appdata.shutdown.get() {
                tracing::debug!("shutting down, requested");
                signal.stop();
                return;
            }

            if appdata.handles.borrow().len() == 0 {
                tracing::debug!("shutting down, no window remaining");
                signal.stop();
                return;
            }

            Data::idle_repaint(handle.clone());
        });

        match res {
            Ok(_) => tracing::info!("wayland event loop completed"),
            Err(cause) => tracing::error!("wayland event loop failed {:?}", cause),
        }
    }

    pub fn quit(&self) {
        self.data.shutdown.set(true);
    }

    pub fn clipboard(&self) -> clipboard::Clipboard {
        clipboard::Clipboard::from(&self.data.clipboard)
    }

    pub fn get_locale() -> String {
        linux::env::locale()
    }
}

impl surfaces::Compositor for Data {
    fn output(&self, id: u32) -> Option<outputs::Meta> {
        self.outputs.borrow().get(&id).cloned()
    }

    fn create_surface(&self) -> wl::Main<WlSurface> {
        self.wl_compositor.create_surface()
    }

    fn create_region(&self) -> wl::Main<WlRegion> {
        self.wl_compositor.create_region()
    }

    fn shared_mem(&self) -> wl::Main<WlShm> {
        self.wl_shm.clone()
    }

    fn get_xdg_positioner(&self) -> wl::Main<XdgPositioner> {
        self.wayland.xdg_base.create_positioner()
    }

    fn get_xdg_surface(&self, s: &wl::Main<WlSurface>) -> wl::Main<xdg_surface::XdgSurface> {
        self.wayland.xdg_base.get_xdg_surface(s)
    }

    fn zwlr_layershell_v1(&self) -> Option<wl::Main<ZwlrLayerShellV1>> {
        self.zwlr_layershell_v1.clone()
    }
}

impl Data {
    pub(crate) fn set_cursor(&self, cursor: &mouse::Cursor) {
        self.pointer.replace(cursor);
    }

    /// Send all pending messages and process all received messages.
    ///
    /// Don't use this once the event loop has started.
    pub(crate) fn sync(&self) -> Result<(), Error> {
        self.wayland
            .queue
            .borrow_mut()
            .sync_roundtrip(&mut (), |evt, _, _| {
                panic!("unexpected wayland event: {evt:?}")
            })
            .map_err(Error::fatal)?;
        Ok(())
    }

    fn current_window_id(&self) -> u64 {
        static DEFAULT: u64 = 0_u64;
        *self.active_surface_id.borrow().front().unwrap_or(&DEFAULT)
    }

    pub(super) fn acquire_current_window(&self) -> Option<WindowHandle> {
        self.handles
            .borrow()
            .get(&self.current_window_id())
            .cloned()
    }

    fn handle_timer_event(&self, _token: TimerToken) {
        // Don't borrow the timers in case the callbacks want to add more.
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
                    tracing::warn!(
                        "received event for surface that doesn't exist any more {:?} {:?}",
                        expired,
                        expired.id()
                    );
                    continue;
                }
            };
            // re-entrancy
            if let Some(data) = win.data() {
                data.handler.borrow_mut().timer(expired.token())
            }
        }

        for (_, win) in self.handles_iter() {
            if let Some(data) = win.data() {
                data.run_deferred_tasks()
            }
        }

        // Get the deadline soonest and queue it.
        if let Some(timer) = self.timers.borrow().peek() {
            self.timer_handle
                .add_timeout(timer.deadline() - now, timer.token());
        }
        // Now flush so the events actually get sent (we don't do this automatically because we
        // aren't in a wayland callback.
        self.wayland.display.flush().unwrap();
    }

    /// Shallow clones surfaces so we can modify it during iteration.
    pub(super) fn handles_iter(&self) -> impl Iterator<Item = (u64, WindowHandle)> {
        self.handles.borrow().clone().into_iter()
    }

    fn idle_repaint(loophandle: calloop::LoopHandle<'_, std::sync::Arc<Data>>) {
        loophandle.insert_idle({
            move |appdata| {
                tracing::trace!("idle processing initiated");
                for (_id, winhandle) in appdata.handles_iter() {
                    winhandle.request_anim_frame();
                    winhandle.run_idle();
                    // if we already flushed this cycle don't flush again.
                    if *appdata.display_flushed.borrow() {
                        tracing::trace!("idle repaint flushing display initiated");
                        if let Err(cause) = appdata.wayland.queue.borrow().display().flush() {
                            tracing::warn!("unable to flush display: {:?}", cause);
                        }
                    }
                }
                tracing::trace!("idle processing completed");
            }
        });
    }
}

impl From<Application> for surfaces::CompositorHandle {
    fn from(app: Application) -> surfaces::CompositorHandle {
        surfaces::CompositorHandle::from(app.data)
    }
}

impl From<std::sync::Arc<Data>> for surfaces::CompositorHandle {
    fn from(data: std::sync::Arc<Data>) -> surfaces::CompositorHandle {
        surfaces::CompositorHandle::direct(
            std::sync::Arc::downgrade(&data) as std::sync::Weak<dyn surfaces::Compositor>
        )
    }
}

#[derive(Debug, Clone)]
pub struct Seat {
    pub(super) wl_seat: wl::Main<WlSeat>,
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
