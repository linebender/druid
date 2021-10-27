use std::convert::TryInto;
use wayland_client as wlc;
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_seat;

use crate::keyboard_types::KeyState;
use crate::text;
use crate::Modifiers;

use super::application::ApplicationData;
use super::surfaces::buffers;
use super::xkb;

pub(super) struct State {
    /// Whether we've currently got keyboard focus.
    focused: bool,
    xkb_context: xkb::Context,
    xkb_keymap: std::cell::RefCell<Option<xkb::Keymap>>,
    xkb_state: std::cell::RefCell<Option<xkb::State>>,
    xkb_mods: std::cell::Cell<Modifiers>,
}

impl State {
    fn focused(&mut self, updated: bool) {
        self.focused = updated;
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            focused: false,
            xkb_context: xkb::Context::new(),
            xkb_keymap: std::cell::RefCell::new(None),
            xkb_state: std::cell::RefCell::new(None),
            xkb_mods: std::cell::Cell::new(Modifiers::empty()),
        }
    }
}

pub struct Manager {
    inner: std::sync::Arc<std::cell::RefCell<State>>,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            inner: std::sync::Arc::new(std::cell::RefCell::new(State::default())),
        }
    }
}

impl Manager {
    pub(super) fn attach(
        &self,
        appdata: std::sync::Arc<ApplicationData>,
        id: u32,
        seat: wlc::Main<wl_seat::WlSeat>,
    ) -> wlc::Main<wl_keyboard::WlKeyboard> {
        let keyboard = seat.get_keyboard();
        keyboard.quick_assign({
            let appdata = appdata.clone();
            let keyboardstate = self.inner.clone();
            move |_, event, _| Manager::consume(&keyboardstate, &appdata, id, event)
        });

        keyboard
    }

    pub(super) fn consume(
        keyboardstate: &std::sync::Arc<std::cell::RefCell<State>>,
        appdata: &std::sync::Arc<ApplicationData>,
        seat: u32,
        event: wl_keyboard::Event,
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

                let state = keyboardstate.borrow_mut();

                // keymap data is '\0' terminated.
                let keymap = state.xkb_context.keymap_from_slice(&keymap_data);
                let keymapstate = keymap.state();

                state.xkb_keymap.replace(Some(keymap));
                state.xkb_state.replace(Some(keymapstate));
            }
            wl_keyboard::Event::Enter { .. } => {
                let winhandle = match appdata.acquire_current_window() {
                    Some(w) => w,
                    None => {
                        tracing::warn!("dropping keyboard events, no window available");
                        return;
                    }
                };

                keyboardstate.borrow_mut().focused(true);
                winhandle.data().map(|data| {
                    // (re-entrancy) call user code
                    data.handler.borrow_mut().got_focus();
                    data.run_deferred_tasks();
                });
            }
            wl_keyboard::Event::Leave { .. } => {
                let winhandle = match appdata.acquire_current_window() {
                    Some(w) => w,
                    None => {
                        tracing::warn!("dropping keyboard events, no window available");
                        return;
                    }
                };

                keyboardstate.borrow_mut().focused(false);
                winhandle.data().map(|data| {
                    // (re-entrancy) call user code
                    data.handler.borrow_mut().lost_focus();
                    data.run_deferred_tasks();
                });
            }
            wl_keyboard::Event::Key { key, state, .. } => {
                let event = keyboardstate
                    .borrow()
                    .xkb_state
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .key_event(
                        key,
                        match state {
                            wl_keyboard::KeyState::Released => KeyState::Up,
                            wl_keyboard::KeyState::Pressed => KeyState::Down,
                            _ => panic!("unrecognised key event"),
                        },
                        keyboardstate.borrow().xkb_mods.get(),
                    );

                if let Some(winhandle) = appdata.acquire_current_window() {
                    winhandle.data().map(|windata| {
                        windata.with_handler({
                            let windata = windata.clone();
                            move |handler| match event.state {
                                KeyState::Up => {
                                    handler.key_up(event);
                                }
                                KeyState::Down => {
                                    let handled = text::simulate_input(
                                        handler,
                                        windata.active_text_input.get(),
                                        event,
                                    );
                                    tracing::trace!(
                                        "key press event {:?} {:?} {:?}",
                                        handled,
                                        key,
                                        windata.active_text_input.get()
                                    );
                                }
                            }
                        });
                    });
                }
            }
            wl_keyboard::Event::Modifiers { .. } => {
                keyboardstate
                    .borrow()
                    .xkb_mods
                    .replace(xkb::event_to_mods(event));
            }
            evt => {
                tracing::warn!("unimplemented keyboard event: {:?}", evt);
            }
        }
    }
}
