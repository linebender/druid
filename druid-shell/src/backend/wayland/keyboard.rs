// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryInto;
use wayland_client as wlc;
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_seat;

use crate::keyboard_types::KeyState;
use crate::text;
use crate::KeyEvent;
use crate::Modifiers;

use super::application::Data;
use super::surfaces::buffers;
use crate::backend::shared::xkb;

#[allow(unused)]
#[derive(Clone)]
struct CachedKeyPress {
    seat: u32,
    serial: u32,
    timestamp: u32,
    key: u32,
    repeat: bool,
    state: wayland_client::protocol::wl_keyboard::KeyState,
    queue: calloop::channel::Sender<KeyEvent>,
}

impl CachedKeyPress {
    fn repeat(&self) -> Self {
        let mut c = self.clone();
        c.repeat = true;
        c
    }
}

#[derive(Debug, Clone)]
struct Repeat {
    rate: std::time::Duration,
    delay: std::time::Duration,
}

impl Default for Repeat {
    fn default() -> Self {
        Self {
            rate: std::time::Duration::from_millis(40),
            delay: std::time::Duration::from_millis(600),
        }
    }
}

struct Keyboard {
    /// Whether we've currently got keyboard focus.
    focused: bool,
    repeat: Repeat,
    last_key_press: Option<CachedKeyPress>,
    xkb_context: xkb::Context,
    xkb_keymap: std::cell::RefCell<Option<xkb::Keymap>>,
    xkb_state: std::cell::RefCell<Option<xkb::State>>,
    xkb_mods: std::cell::Cell<Modifiers>,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            focused: false,
            repeat: Repeat::default(),
            last_key_press: None,
            xkb_context: xkb::Context::new(),
            xkb_keymap: std::cell::RefCell::new(None),
            xkb_state: std::cell::RefCell::new(None),
            xkb_mods: std::cell::Cell::new(Modifiers::empty()),
        }
    }
}

impl Keyboard {
    fn focused(&mut self, updated: bool) {
        self.focused = updated;
    }

    fn repeat(&mut self, u: Repeat) {
        self.repeat = u;
    }

    fn replace_last_key_press(&mut self, u: Option<CachedKeyPress>) {
        self.last_key_press = u;
    }

    fn release_last_key_press(&self, current: &CachedKeyPress) -> Option<CachedKeyPress> {
        match &self.last_key_press {
            None => None, // nothing to do.
            Some(last) => {
                if last.serial >= current.serial {
                    return Some(last.clone());
                }
                if last.key != current.key {
                    return Some(last.clone());
                }
                None
            }
        }
    }

    fn keystroke<'a>(&'a mut self, keystroke: &'a CachedKeyPress) {
        let keystate = match keystroke.state {
            wl_keyboard::KeyState::Released => {
                self.replace_last_key_press(self.release_last_key_press(keystroke));
                KeyState::Up
            }
            wl_keyboard::KeyState::Pressed => {
                self.replace_last_key_press(Some(keystroke.repeat()));
                KeyState::Down
            }
            _ => panic!("unrecognised key event"),
        };

        let mut event = self.xkb_state.borrow_mut().as_mut().unwrap().key_event(
            keystroke.key,
            keystate,
            keystroke.repeat,
        );
        event.mods = self.xkb_mods.get();

        if let Err(cause) = keystroke.queue.send(event) {
            tracing::error!("failed to send Druid key event: {:?}", cause);
        }
    }

    fn consume(
        &mut self,
        seat: u32,
        event: wl_keyboard::Event,
        keyqueue: calloop::channel::Sender<KeyEvent>,
    ) {
        tracing::trace!("consume {:?} -> {:?}", seat, event);
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                if !matches!(format, wl_keyboard::KeymapFormat::XkbV1) {
                    panic!("only xkb keymap supported for now");
                }

                // TODO to test memory ownership we copy the memory. That way we can deallocate it
                // and see if we get a segfault.
                let keymap_data = unsafe {
                    buffers::Mmap::from_raw_private(
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
                let keymapstate = keymap.state();

                self.xkb_keymap.replace(Some(keymap));
                self.xkb_state.replace(Some(keymapstate));
            }
            wl_keyboard::Event::Enter { .. } => {
                self.focused(true);
            }
            wl_keyboard::Event::Leave { .. } => {
                self.focused(false);
            }
            wl_keyboard::Event::Key {
                serial,
                time,
                state,
                key,
            } => {
                tracing::trace!(
                    "key stroke registered {:?} {:?} {:?} {:?}",
                    time,
                    serial,
                    key,
                    state
                );
                self.keystroke(&CachedKeyPress {
                    repeat: false,
                    seat,
                    serial,
                    timestamp: time,
                    key: key + 8, // TODO: understand the magic 8.
                    state,
                    queue: keyqueue,
                })
            }
            wl_keyboard::Event::Modifiers { .. } => {
                self.xkb_mods.replace(event_to_mods(event));
            }
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                tracing::trace!("keyboard repeat info received {:?} {:?}", rate, delay);
                self.repeat(Repeat {
                    rate: std::time::Duration::from_millis((1000 / rate) as u64),
                    delay: std::time::Duration::from_millis(delay as u64),
                });
            }
            evt => {
                tracing::warn!("unimplemented keyboard event: {:?}", evt);
            }
        }
    }
}

pub(super) struct State {
    apptx: calloop::channel::Sender<KeyEvent>,
    apprx: std::cell::RefCell<Option<calloop::channel::Channel<KeyEvent>>>,
    tx: calloop::channel::Sender<(u32, wl_keyboard::Event, calloop::channel::Sender<KeyEvent>)>,
}

impl Default for State {
    fn default() -> Self {
        let (apptx, apprx) = calloop::channel::channel::<KeyEvent>();
        let (tx, rx) = calloop::channel::channel::<(
            u32,
            wl_keyboard::Event,
            calloop::channel::Sender<KeyEvent>,
        )>();
        let state = Self {
            apptx,
            apprx: std::cell::RefCell::new(Some(apprx)),
            tx,
        };

        std::thread::spawn(move || {
            let mut eventloop: calloop::EventLoop<(calloop::LoopSignal, Keyboard)> =
                calloop::EventLoop::try_new()
                    .expect("failed to initialize the keyboard event loop!");
            let signal = eventloop.get_signal();
            let handle = eventloop.handle();
            let repeat = calloop::timer::Timer::<CachedKeyPress>::new().unwrap();
            handle
                .insert_source(rx, {
                    let repeater = repeat.handle();
                    move |event, _ignored, state| {
                        let event = match event {
                            calloop::channel::Event::Closed => {
                                tracing::info!("keyboard event loop closed shutting down");
                                state.0.stop();
                                return;
                            }
                            calloop::channel::Event::Msg(keyevent) => keyevent,
                        };
                        state.1.consume(event.0, event.1, event.2);
                        match &state.1.last_key_press {
                            None => repeater.cancel_all_timeouts(),
                            Some(cached) => {
                                repeater.cancel_all_timeouts();
                                repeater.add_timeout(state.1.repeat.delay, cached.clone());
                            }
                        };
                    }
                })
                .unwrap();

            // generate repeat keypresses.
            handle
                .insert_source(repeat, |event, timer, state| {
                    timer.add_timeout(state.1.repeat.rate, event.clone());
                    state.1.keystroke(&event);
                })
                .unwrap();

            tracing::debug!("keyboard event loop initiated");
            eventloop
                .run(
                    std::time::Duration::from_secs(60),
                    &mut (signal, Keyboard::default()),
                    |_ignored| {
                        tracing::trace!("keyboard event loop idle");
                    },
                )
                .expect("keyboard event processing failed");
            tracing::debug!("keyboard event loop completed");
        });

        state
    }
}

struct ModMap(u32, Modifiers);

impl ModMap {
    fn merge(self, m: Modifiers, mods: u32, locked: u32) -> Modifiers {
        if self.0 & mods == 0 && self.0 & locked == 0 {
            return m;
        }

        m | self.1
    }
}

const MOD_SHIFT: ModMap = ModMap(1, Modifiers::SHIFT);
const MOD_CAP_LOCK: ModMap = ModMap(2, Modifiers::CAPS_LOCK);
const MOD_CTRL: ModMap = ModMap(4, Modifiers::CONTROL);
const MOD_ALT: ModMap = ModMap(8, Modifiers::ALT);
const MOD_NUM_LOCK: ModMap = ModMap(16, Modifiers::NUM_LOCK);
const MOD_META: ModMap = ModMap(64, Modifiers::META);

pub fn event_to_mods(event: wl_keyboard::Event) -> Modifiers {
    match event {
        wl_keyboard::Event::Modifiers {
            mods_depressed,
            mods_locked,
            ..
        } => {
            let mods = Modifiers::empty();
            let mods = MOD_SHIFT.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_CAP_LOCK.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_CTRL.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_ALT.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_NUM_LOCK.merge(mods, mods_depressed, mods_locked);

            MOD_META.merge(mods, mods_depressed, mods_locked)
        }
        _ => Modifiers::empty(),
    }
}

pub struct Manager {
    inner: std::sync::Arc<State>,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            inner: std::sync::Arc::new(State::default()),
        }
    }
}

impl Manager {
    pub(super) fn attach(
        &self,
        id: u32,
        seat: wlc::Main<wl_seat::WlSeat>,
    ) -> wlc::Main<wl_keyboard::WlKeyboard> {
        let keyboard = seat.get_keyboard();
        keyboard.quick_assign({
            let tx = self.inner.tx.clone();
            let queue = self.inner.apptx.clone();
            move |_, event, _| {
                if let Err(cause) = tx.send((id, event, queue.clone())) {
                    tracing::error!("failed to transmit keyboard event {:?}", cause);
                };
            }
        });

        keyboard
    }

    // TODO turn struct into a calloop event source.
    pub(super) fn events(&self, handle: &calloop::LoopHandle<std::sync::Arc<Data>>) {
        let rx = self.inner.apprx.borrow_mut().take().unwrap();
        handle
            .insert_source(rx, {
                move |evt, _ignored, appdata| {
                    let evt = match evt {
                        calloop::channel::Event::Msg(e) => e,
                        calloop::channel::Event::Closed => {
                            tracing::info!("keyboard events receiver closed");
                            return;
                        }
                    };

                    if let Some(winhandle) = appdata.acquire_current_window() {
                        if let Some(windata) = winhandle.data() {
                            windata.with_handler({
                                let windata = windata.clone();
                                move |handler| match evt.state {
                                    KeyState::Up => {
                                        handler.key_up(evt.clone());
                                        tracing::trace!(
                                            "key press event up {:?} {:?}",
                                            evt,
                                            windata.active_text_input.get()
                                        );
                                    }
                                    KeyState::Down => {
                                        let handled = text::simulate_input(
                                            handler,
                                            windata.active_text_input.get(),
                                            evt.clone(),
                                        );
                                        tracing::trace!(
                                            "key press event down {:?} {:?} {:?}",
                                            handled,
                                            evt,
                                            windata.active_text_input.get()
                                        );
                                    }
                                }
                            });
                        }
                    }
                }
            })
            .unwrap();
    }
}
