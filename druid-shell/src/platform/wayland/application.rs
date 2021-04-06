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
    buffer::Mmap,
    clipboard::Clipboard,
    error::Error,
    events::WaylandSource,
    pointer::{MouseEvtKind, PointerEvent},
    window::WindowData,
    xkb,
};
use crate::{
    application::AppHandler, keyboard_types::KeyState, kurbo::Point, platform::shared::Timer,
    TimerToken, WinHandler,
};

use calloop::{
    timer::{Timer as CalloopTimer, TimerHandle},
    EventLoop,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BinaryHeap},
    convert::TryInto,
    num::NonZeroI32,
    rc::Rc,
    time::{Duration, Instant},
};
use wayland_client::{
    self as wl,
    protocol::{
        wl_compositor::WlCompositor,
        wl_keyboard::{self, WlKeyboard},
        wl_output::{self, Subpixel, Transform, WlOutput},
        wl_pointer::{self, WlPointer},
        wl_seat::{self, WlSeat},
        wl_shm::{self, WlShm},
    },
    Proxy,
};
use wayland_cursor::CursorTheme;
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
    xdg_shell::client::xdg_wm_base::{self, XdgWmBase},
};

#[derive(Clone)]
pub struct Application {
    pub(crate) data: Rc<ApplicationData>,
}

pub(crate) struct ApplicationData {
    pub(crate) xkb_context: xkb::Context,
    pub(crate) xkb_keymap: RefCell<Option<xkb::Keymap>>,
    // TODO should this be per-surface??
    pub(crate) xkb_state: RefCell<Option<xkb::State>>,
    pub(crate) wl_server: wl::Display,
    pub(crate) event_queue: Rc<RefCell<wl::EventQueue>>,
    // Wayland globals
    pub(crate) globals: wl::GlobalManager,
    pub(crate) xdg_base: wl::Main<XdgWmBase>,
    pub(crate) zxdg_decoration_manager_v1: wl::Main<ZxdgDecorationManagerV1>,
    pub(crate) wl_compositor: wl::Main<WlCompositor>,
    pub(crate) wl_shm: wl::Main<WlShm>,
    pub(crate) cursor_theme: RefCell<CursorTheme>,
    /// A map of wayland object IDs to outputs.
    ///
    /// Wayland will update this if the output change. Keep a record of the `Instant` you last
    /// observed a change, and use `Output::changed` to see if there are any newer changes.
    ///
    /// It's a BTreeMap so the ordering is consistent when enumerating outputs (not sure if this is
    /// necessary, but it negligable cost).
    pub(crate) outputs: Rc<RefCell<BTreeMap<u32, Output>>>,
    pub(crate) seats: Rc<RefCell<BTreeMap<u32, Rc<RefCell<Seat>>>>>,
    /// Handles to any surfaces that have been created.
    ///
    /// This is where the window data is owned. Other handles should be weak.
    pub(crate) surfaces: RefCell<im::OrdMap<u32, Rc<WindowData>>>,

    /// Available pixel formats
    pub(crate) formats: RefCell<Vec<wl_shm::Format>>,
    /// Close flag
    pub(crate) shutdown: Cell<bool>,
    /// The currently active surface, if any (by wayland object ID)
    pub(crate) active_surface_id: Cell<Option<NonZeroI32>>,
    // Stuff for timers
    /// A calloop event source for timers. We always set it to fire at the next set timer, if any.
    pub(crate) timer_handle: TimerHandle<TimerToken>,
    /// We stuff this here until the event loop, then `take` it and use it.
    timer_source: RefCell<Option<CalloopTimer<TimerToken>>>,
    /// Currently pending timers
    ///
    /// The extra data is the surface this timer is for.
    pub(crate) timers: RefCell<BinaryHeap<Timer<u32>>>,
}

impl Application {
    pub fn new() -> Result<Self, Error> {
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
        let globals = wl::GlobalManager::new_with_cb(
            &attached_server,
            move |event, registry, _| match event {
                wl::GlobalEvent::New {
                    id,
                    interface,
                    version,
                } => {
                    //println!("{}@{} - {}", interface, version, id);
                    if interface.as_str() == "wl_output" && version >= 3 {
                        let output = registry.bind::<WlOutput>(3, id);

                        let output = Output::new(output);
                        let output_id = output.id();
                        output.wl_output.quick_assign(with_cloned!(weak_outputs; move |_, event, _| {
                                weak_outputs
                                .upgrade()
                                .unwrap()
                                .borrow_mut()
                                .get_mut(&output_id)
                                .expect(
                                    "internal: wayland sent an event for an output that doesn't exist",
                                )
                                .process_event(event)
                            }));
                        let prev_output = weak_outputs
                            .upgrade()
                            .unwrap()
                            .borrow_mut()
                            .insert(output_id, output);
                        assert!(
                            prev_output.is_none(),
                            "internal: wayland should always use new IDs"
                        );
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
                wl::GlobalEvent::Removed { id, interface } if interface.as_str() == "wl_output" => {
                    let removed = weak_outputs
                        .upgrade()
                        .unwrap()
                        .borrow_mut()
                        .remove(&id)
                        .expect("internal: wayland removed an output that doesn't exist");
                    removed.wl_output.release();
                }
                _ => (), // ignore other interfaces
            },
        );

        // do a round trip to make sure we have all the globals
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| unreachable!())
            .map_err(Error::fatal)?;

        let mut globals_list = globals.list();
        globals_list.sort_by(|(_, name1, version1), (_, name2, version2)| {
            name1.cmp(name2).then(version1.cmp(version2))
        });
        for (id, name, version) in globals_list.into_iter() {
            //println!("{}@{} - {}", name, version, id);
        }

        let xdg_base = globals
            .instantiate_exact::<XdgWmBase>(2)
            .map_err(|e| Error::global("xdg_wm_base", 2, e))?;
        let zxdg_decoration_manager_v1 = globals
            .instantiate_exact::<ZxdgDecorationManagerV1>(1)
            .map_err(|e| Error::global("zxdg_decoration_manager_v1", 1, e))?;
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
        xdg_base.quick_assign(|xdg_base, event, _| match event {
            xdg_wm_base::Event::Ping { serial } => xdg_base.pong(serial),
            _ => (),
        });

        let xkb_context = xkb::Context::new();
        //xkb_context.set_log_level(log::Level::Trace);

        let timer_source = CalloopTimer::new().unwrap();
        let timer_handle = timer_source.handle();

        // TODO the choice of size needs more refinement, it should probably be the size needed to
        // draw sharp cursors on the largest scaled monitor.
        let cursor_theme = CursorTheme::load(64, &wl_shm);

        // We need to have keyboard events set up for our seats before the next roundtrip.
        let app_data = Rc::new(ApplicationData {
            xkb_context,
            xkb_keymap: RefCell::new(None),
            xkb_state: RefCell::new(None),
            wl_server,
            event_queue: Rc::new(RefCell::new(event_queue)),
            globals,
            xdg_base,
            zxdg_decoration_manager_v1,
            wl_compositor,
            wl_shm: wl_shm.clone(),
            cursor_theme: RefCell::new(cursor_theme),
            outputs,
            seats,
            surfaces: RefCell::new(im::OrdMap::new()),
            formats: RefCell::new(vec![]),
            shutdown: Cell::new(false),
            active_surface_id: Cell::new(None),
            timer_handle,
            timer_source: RefCell::new(Some(timer_source)),
            timers: RefCell::new(BinaryHeap::new()),
        });

        // Collect the supported image formats.
        wl_shm.quick_assign(with_cloned!(app_data; move |_, event, _| {
            match event {
                wl_shm::Event::Format { format } => app_data.formats.borrow_mut().push(format),
                _ => (), // ignore other messages
            }
        }));

        //println!("{:?}", app_data.seats.borrow());

        // Setup seat event listeners with our application
        for (id, seat) in app_data.seats.borrow().iter() {
            let id = *id; // move into closure.
            let wl_seat = seat.borrow().wl_seat.clone();
            wl_seat.quick_assign(with_cloned!(seat, app_data; move |_, event, _| {
                let mut seat = seat.borrow_mut();
                match event {
                    wl_seat::Event::Capabilities { capabilities } => {
                        seat.capabilities = capabilities;
                        if capabilities.contains(wl_seat::Capability::Keyboard)
                            && seat.keyboard.is_none()
                        {
                            let keyboard = seat.wl_seat.get_keyboard();
                            let app = app_data.clone();
                            keyboard.quick_assign(move |_, event, _| {
                                app.handle_keyboard_event(id, event);
                            });
                            seat.keyboard = Some(keyboard);
                        }
                        if capabilities.contains(wl_seat::Capability::Pointer)
                            && seat.pointer.is_none()
                        {
                            let pointer = seat.wl_seat.get_pointer();
                            let app = app_data.clone();
                            let pointer_clone = pointer.detach();
                            pointer.quick_assign(move |_, event, _| {
                                let pointer_clone = pointer_clone.clone();
                                app.handle_pointer_event(pointer_clone, event);
                            });
                            seat.pointer = Some(pointer);
                        }
                        // Dont worry if they go away - we will just stop receiving events. If the
                        // capability comes back we will start getting events again.
                        seat.last_update = Instant::now();
                    }
                    wl_seat::Event::Name { name } => {
                        seat.name = name;
                        seat.last_update = Instant::now();
                    }
                    _ => (), // ignore future events
                }
            }));
        }
        /*
        new_seat.quick_assign(move |_, event, _| {
        });
        */

        // Let wayland finish setup before we allow the client to start creating windows etc.
        app_data.sync()?;

        Ok(Application { data: app_data })
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        // NOTE if we want to call this function more than once, we will need to put the timer
        // source back.
        let timer_source = self.data.timer_source.borrow_mut().take().unwrap();
        // flush pending events (otherwise anything we submitted since sync will never be sent)
        self.data.wl_server.flush().unwrap();
        // Use calloop so we can epoll both wayland events and others (e.g. timers)
        let mut event_loop = EventLoop::try_new().expect("failed to initialize calloop event loop");
        let handle = event_loop.handle();
        let wayland_dispatcher =
            WaylandSource::new(self.data.event_queue.clone()).into_dispatcher();
        handle.register_dispatcher(wayland_dispatcher).unwrap();
        let app_data = self.data.clone();
        handle
            .insert_source(timer_source, move |token, handle, &mut ()| {
                app_data.handle_timer_event(token);
            })
            .unwrap();
        let signal = event_loop.get_signal();
        event_loop
            .run(Duration::from_millis(20), &mut (), move |&mut ()| {
                if self.data.shutdown.get() {
                    signal.stop();
                }
            })
            .unwrap();
    }

    pub fn quit(&self) {
        self.data.shutdown.set(true);
    }

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO
        "en_US".into()
    }
}

impl ApplicationData {
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

    fn handle_keyboard_event(&self, seat_id: u32, event: wl_keyboard::Event) {
        use wl_keyboard::{Event, KeyState as WlKeyState, KeymapFormat};
        // TODO need to keep the serial around for certain requests.
        match event {
            Event::Keymap { format, fd, size } => {
                if !matches!(format, KeymapFormat::XkbV1) {
                    panic!("only xkb keymap supported for now");
                }
                // TODO to test memory ownership we copy the memory. That way we can deallocate it
                // and see if we get a segfault.
                let keymap_data = unsafe {
                    Mmap::from_raw_private(
                        fd,
                        size.try_into().unwrap(),
                        0,
                        size.try_into().unwrap(),
                    )
                    .unwrap()
                    .as_ref()
                    .to_vec()
                };
                // keymap data is '\0' terminated.
                let keymap = self.xkb_context.keymap_from_slice(&keymap_data);
                let state = keymap.state();
                *self.xkb_keymap.borrow_mut() = Some(keymap);
                *self.xkb_state.borrow_mut() = Some(state);
            }
            Event::Enter {
                serial,
                surface,
                keys,
            } => {
                let data = self
                    .find_surface(Proxy::from(surface).id())
                    .expect("received a pointer event for a non-existant surface");
                data.keyboard_focus.set(true);
                // (re-entrancy) call user code
                data.handler.borrow_mut().got_focus();

                data.run_deferred_tasks();
            }
            Event::Leave { serial, surface } => {
                let data = self
                    .find_surface(Proxy::from(surface).id())
                    .expect("received a pointer event for a non-existant surface");
                data.keyboard_focus.set(false);
                // (re-entrancy) call user code
                data.handler.borrow_mut().lost_focus();
                data.run_deferred_tasks();
            }
            Event::Key {
                serial,
                time,
                key,
                state,
            } => {
                let event = self.xkb_state.borrow().as_ref().unwrap().key_event(
                    key,
                    match state {
                        WlKeyState::Released => KeyState::Up,
                        WlKeyState::Pressed => KeyState::Down,
                        _ => panic!("unrecognised key event"),
                    },
                );
                // This clone is necessary because user methods might add more surfaces, which
                // would then be inserted here which would be 1 mut 1 immut borrows which is not
                // allowed.
                for (_, data) in self.surfaces_iter() {
                    if data.keyboard_focus.get() {
                        match event.state {
                            KeyState::Down => {
                                // TODO what do I do if the key event is handled? Do I not update
                                // the xkb state?
                                data.handler.borrow_mut().key_down(event.clone());
                            }
                            KeyState::Up => data.handler.borrow_mut().key_up(event.clone()),
                        }
                    }
                    data.run_deferred_tasks();
                }
            }
            Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => {
                // Ignore this event for now and handle modifiers in user code. This might be
                // suboptimal and need revisiting in the future.
                //surface.check_for_scheduled_paint();
            }
            Event::RepeatInfo { rate, delay } => {
                // TODO actually store/use this info
                println!("Requested repeat rate={} delay={}", rate, delay);
            }
            evt => {
                log::warn!("Unhandled keybaord event: {:?}", evt);
            }
        }
    }

    /// `seat_id` is the object ID for the seat.
    fn handle_pointer_event(&self, wl_pointer: WlPointer, event: wl_pointer::Event) {
        use wl_pointer::Event;
        match event {
            Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                let data = self
                    .find_surface(Proxy::from(surface).id())
                    .expect("received a pointer event for a non-existant surface");
                // TODO some investigation will be needed to deduce the space `surface_x` and
                // `surface_y` are relative to.
                data.init_pointer(wl_pointer, serial);
                // cannot fail (we just set it to Some)
                let mut _pointer = data.pointer.borrow_mut();
                let pointer = _pointer.as_mut().unwrap();
                // No mouse enter event, but we know the position so we can issue a mouse move.
                let pos = Point::new(surface_x, surface_y);
                pointer.push(PointerEvent::Motion(pos));
            }
            Event::Leave { serial, surface } => {
                let data = self
                    .find_surface(Proxy::from(surface).id())
                    .expect("received a pointer event for a non-existant surface");
                if let Some(pointer) = data.pointer.borrow_mut().as_mut() {
                    pointer.push(PointerEvent::Leave);
                };
            }
            Event::Motion {
                time,
                surface_x,
                surface_y,
            } => {
                let pos = Point::new(surface_x, surface_y);
                for (_, data) in self.surfaces_iter() {
                    if let Some(pointer) = data.pointer.borrow_mut().as_mut() {
                        pointer.push(PointerEvent::Motion(pos));
                    }
                }
            }
            Event::Button {
                serial,
                time,
                button,
                state,
            } => {
                for (_, data) in self.surfaces_iter() {
                    if let Some(pointer) = data.pointer.borrow_mut().as_mut() {
                        pointer.push(PointerEvent::Button { button, state });
                    }
                }
            }
            Event::Axis { time, axis, value } => {
                for (_, data) in self.surfaces_iter() {
                    if let Some(pointer) = data.pointer.borrow_mut().as_mut() {
                        pointer.push(PointerEvent::Axis { axis, value });
                    }
                }
            }
            Event::Frame => {
                for (_, data) in self.surfaces_iter() {
                    // Wait until we're outside the loop, then drop the pointer state.
                    let mut have_left = false;
                    while let Some(event) = data.pop_pointer_event() {
                        // (re-entrancy)
                        match event {
                            MouseEvtKind::Move(evt) => data.handler.borrow_mut().mouse_move(&evt),
                            MouseEvtKind::Up(evt) => data.handler.borrow_mut().mouse_up(&evt),
                            MouseEvtKind::Down(evt) => data.handler.borrow_mut().mouse_down(&evt),
                            MouseEvtKind::Leave => {
                                have_left = true;
                                data.handler.borrow_mut().mouse_leave();
                            }
                            MouseEvtKind::Wheel(evt) => data.handler.borrow_mut().wheel(&evt),
                        }
                    }
                    if have_left {
                        *data.pointer.borrow_mut() = None;
                    }
                    data.run_deferred_tasks();
                }
            }
            evt => {
                log::warn!("Unhandled pointer event: {:?}", evt);
            }
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
        for timer in expired_timers {
            let surface = match self.surfaces.borrow().get(&timer.data).cloned() {
                Some(s) => s,
                None => {
                    // NOTE this might be expected
                    log::warn!("Received event for surface that doesn't exist any more");
                    continue;
                }
            };
            // re-entrancy
            surface.handler.borrow_mut().timer(timer.token());
        }
        for (_, data) in self.surfaces_iter() {
            data.run_deferred_tasks();
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

    fn find_surface(&self, id: u32) -> Option<Rc<WindowData>> {
        self.surfaces.borrow().get(&id).cloned()
    }

    /// Shallow clones surfaces so we can modify it during iteration.
    fn surfaces_iter(&self) -> impl Iterator<Item = (u32, Rc<WindowData>)> {
        // make sure the borrow gets dropped.
        let surfaces = {
            let surfaces = self.surfaces.borrow();
            surfaces.clone()
        };
        surfaces.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct Output {
    wl_output: wl::Main<WlOutput>,
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

impl Output {
    // All the stuff before `current_mode` will be filled out immediately after creation, so these
    // dummy values will never be observed.
    fn new(wl_output: wl::Main<WlOutput>) -> Self {
        Output {
            wl_output,
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
        Proxy::from(self.wl_output.detach()).id()
    }

    /// Incorporate update data from the server for this output.
    fn process_event(&mut self, evt: wl_output::Event) {
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
                self.physical_width = physical_width;
                self.physical_height = physical_height;
                self.subpixel = subpixel;
                self.make = make;
                self.model = model;
                self.transform = transform;

                self.update_in_progress = true;
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
            }
            wl_output::Event::Scale { factor } => {
                self.scale = factor;
                self.update_in_progress = true;
            }
            _ => (), // ignore possible future events
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
    width: i32,
    height: i32,
    refresh: i32,
}

#[derive(Debug, Clone)]
pub struct Seat {
    wl_seat: wl::Main<WlSeat>,
    name: String,
    capabilities: wl_seat::Capability,
    keyboard: Option<wl::Main<WlKeyboard>>,
    pointer: Option<wl::Main<WlPointer>>,
    // TODO touch
    /// Lets us work out if things have changed since we last observed the output.
    last_update: Instant,
}

impl Seat {
    fn new(wl_seat: wl::Main<WlSeat>) -> Self {
        Self {
            wl_seat,
            name: "".into(),
            capabilities: wl_seat::Capability::empty(),
            last_update: Instant::now(),
            keyboard: None,
            pointer: None,
        }
    }
}
