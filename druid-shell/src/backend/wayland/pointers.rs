// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::collections::VecDeque;
use wayland_client::protocol::wl_pointer;
use wayland_client::protocol::wl_surface::{self, WlSurface};
use wayland_client::{self as wl};
use wayland_cursor::CursorImageBuffer;
use wayland_cursor::CursorTheme;

use crate::keyboard::Modifiers;
use crate::kurbo::{Point, Vec2};
use crate::mouse;

use super::application::Data;

// Button constants (linux specific)
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;

// used to keep track of click event counts.
#[derive(Debug, Clone)]
struct ClickDebouncer {
    timestamp: std::time::Instant,
    count: u8,
    previous: mouse::MouseButton,
}

impl Default for ClickDebouncer {
    fn default() -> Self {
        Self {
            timestamp: std::time::Instant::now(),
            count: 1,
            previous: mouse::MouseButton::None,
        }
    }
}

impl ClickDebouncer {
    // this threshold was arbitrarily chosen based on experimention.
    // there is likely a better default based on research to use.
    // during experimentation this allowed one to get to around 4 clicks.
    // but likely heavily dependent on the machine.
    const THRESHOLD: std::time::Duration = std::time::Duration::from_millis(500);

    fn reset(ts: std::time::Instant, btn: mouse::MouseButton) -> Self {
        Self {
            timestamp: ts,
            count: 1,
            previous: btn,
        }
    }

    fn debounce(&mut self, current: MouseEvtKind) -> MouseEvtKind {
        let ts = std::time::Instant::now();

        // reset counting and button.
        if self.timestamp + ClickDebouncer::THRESHOLD < ts {
            *self = ClickDebouncer::default();
        }

        match current {
            MouseEvtKind::Up(mut evt) if self.previous == evt.button => {
                evt.count = self.count;
                MouseEvtKind::Up(evt)
            }
            MouseEvtKind::Down(mut evt) if self.previous == evt.button => {
                self.count += 1;
                evt.count = self.count;
                MouseEvtKind::Down(evt)
            }
            MouseEvtKind::Down(evt) if self.previous != evt.button => {
                *self = ClickDebouncer::reset(ts, evt.button);
                MouseEvtKind::Down(evt)
            }
            MouseEvtKind::Leave => {
                *self = ClickDebouncer::reset(ts, mouse::MouseButton::None);
                current
            }
            _ => current,
        }
    }
}

/// Collect up mouse events then emit them together on a pointer frame.
pub(crate) struct Pointer {
    /// The image surface which contains the cursor image.
    pub(crate) cursor_surface: wl::Main<WlSurface>,
    /// Events that have occurred since the last frame.
    pub(crate) queued_events: std::cell::RefCell<VecDeque<PointerEvent>>,
    /// Currently pressed buttons
    buttons: std::cell::RefCell<mouse::MouseButtons>,
    /// Current position
    pos: std::cell::Cell<Point>,
    wl_pointer: std::cell::RefCell<Option<wl_pointer::WlPointer>>,
    // used to keep track of the current clicking
    clickevent: std::cell::RefCell<ClickDebouncer>,
    /// cursor theme data.
    theme: std::cell::RefCell<CursorTheme>,
    /// Cache the current cursor, so we can see if it changed
    current_cursor: std::cell::RefCell<mouse::Cursor>,
}

/// Raw wayland pointer events.
#[derive(Debug)]
pub(crate) enum PointerEvent {
    /// Mouse moved/entered
    Motion {
        pointer: wl_pointer::WlPointer,
        point: Point,
    },
    /// Mouse button pressed/released
    Button {
        button: u32,
        state: wl_pointer::ButtonState,
    },
    /// Axis movement
    Axis { axis: wl_pointer::Axis, value: f64 },
    /// Mouse left
    Leave,
}

/// An enum that we will convert into the different callbacks.
#[derive(Debug)]
pub(crate) enum MouseEvtKind {
    Move(mouse::MouseEvent),
    Up(mouse::MouseEvent),
    Down(mouse::MouseEvent),
    Leave,
    Wheel(mouse::MouseEvent),
}

#[allow(unused)]
impl Pointer {
    /// Create a new pointer
    pub fn new(theme: CursorTheme, cursor: wl::Main<WlSurface>) -> Self {
        // ignore all events
        cursor.quick_assign(|a1, event, a2| {
            tracing::trace!("pointer surface event {:?} {:?} {:?}", a1, event, a2);
        });

        Pointer {
            theme: std::cell::RefCell::new(theme),
            buttons: std::cell::RefCell::new(mouse::MouseButtons::new()),
            pos: std::cell::Cell::new(Point::ZERO), // will get set before we emit any events
            queued_events: std::cell::RefCell::new(VecDeque::with_capacity(3)), // should be enough most of the time
            cursor_surface: cursor,
            wl_pointer: std::cell::RefCell::new(None),
            current_cursor: std::cell::RefCell::new(mouse::Cursor::Arrow),
            clickevent: std::cell::RefCell::new(ClickDebouncer::default()),
        }
    }

    pub fn attach(&self, current: wl_pointer::WlPointer) {
        tracing::trace!("attaching pointer reference {:?}", current);
        self.wl_pointer.replace(Some(current));
    }

    #[inline]
    pub fn push(&self, event: PointerEvent) {
        self.queued_events.borrow_mut().push_back(event);
    }

    #[inline]
    pub fn pop(&self) -> Option<PointerEvent> {
        self.queued_events.borrow_mut().pop_front()
    }

    #[inline]
    pub fn cursor(&self) -> &WlSurface {
        &self.cursor_surface
    }

    pub fn replace(&self, cursor: &mouse::Cursor) {
        let current = self.current_cursor.borrow().clone();
        let cursor = cursor.clone();

        // Setting a new cursor involves communicating with the server, so don't do it if we
        // don't have to.
        if current == cursor {
            return;
        }

        let b = self.wl_pointer.borrow_mut();
        let wl_pointer = match &*b {
            None => return,
            Some(p) => p,
        };

        tracing::trace!("replacing cursor {:?} -> {:?}", current, cursor);
        let buffer = match self.get_cursor_buffer(&cursor) {
            None => return,
            Some(b) => b,
        };

        let (hot_x, hot_y) = buffer.hotspot();
        self.current_cursor.replace(cursor);
        wl_pointer.set_cursor(0, Some(&self.cursor_surface), hot_x as i32, hot_y as i32);
        self.cursor_surface.attach(Some(&*buffer), 0, 0);

        if self.cursor_surface.as_ref().version() >= wl_surface::REQ_DAMAGE_BUFFER_SINCE {
            self.cursor_surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
        } else {
            self.cursor_surface.damage(0, 0, i32::MAX, i32::MAX);
        }

        self.cursor_surface.commit();
    }

    fn get_cursor_buffer(&self, cursor: &mouse::Cursor) -> Option<CursorImageBuffer> {
        #[allow(deprecated)]
        match cursor {
            mouse::Cursor::Arrow => self.unpack_image_buffer("left_ptr"),
            mouse::Cursor::IBeam => self.unpack_image_buffer("xterm"),
            mouse::Cursor::Crosshair => self.unpack_image_buffer("cross"),
            mouse::Cursor::OpenHand => self.unpack_image_buffer("openhand"),
            mouse::Cursor::NotAllowed => self.unpack_image_buffer("X_cursor"),
            mouse::Cursor::ResizeLeftRight => self.unpack_image_buffer("row-resize"),
            mouse::Cursor::ResizeUpDown => self.unpack_image_buffer("col-resize"),
            mouse::Cursor::Pointer => self.unpack_image_buffer("pointer"),
            mouse::Cursor::Custom(_) => {
                tracing::warn!("custom cursors not implemented");
                self.unpack_image_buffer("left_ptr")
            }
        }
    }

    // Just use the first image, people using animated cursors have already made bad life
    // choices and shouldn't expect it to work.
    fn unpack_image_buffer(&self, name: &str) -> Option<CursorImageBuffer> {
        self.theme
            .borrow_mut()
            .get_cursor(name)
            .map(|c| c[c.frame_and_duration(0).frame_index].clone())
    }

    pub(super) fn consume(
        appdata: std::sync::Arc<Data>,
        source: wl_pointer::WlPointer,
        event: wl_pointer::Event,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface,
                surface_x,
                surface_y,
                ..
            } => {
                appdata.pointer.push(PointerEvent::Motion {
                    point: Point::new(surface_x, surface_y),
                    pointer: source,
                });
            }
            wl_pointer::Event::Leave { surface, .. } => {
                appdata.pointer.push(PointerEvent::Leave);
            }
            wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                appdata.pointer.push(PointerEvent::Motion {
                    point: Point::new(surface_x, surface_y),
                    pointer: source,
                });
            }
            wl_pointer::Event::Button { button, state, .. } => {
                appdata.pointer.push(PointerEvent::Button { button, state });
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                appdata.pointer.push(PointerEvent::Axis { axis, value });
            }
            wl_pointer::Event::Frame => {
                let winhandle = match appdata.acquire_current_window().and_then(|w| w.data()) {
                    Some(w) => w,
                    None => {
                        tracing::warn!("dropping mouse events, no window available");
                        appdata.pointer.queued_events.borrow_mut().clear();
                        return;
                    }
                };
                let mut winhandle = winhandle.handler.borrow_mut();

                // (re-entrancy) call user code
                while let Some(event) = appdata.pointer.dequeue() {
                    match event {
                        MouseEvtKind::Move(evt) => winhandle.mouse_move(&evt),
                        MouseEvtKind::Up(evt) => winhandle.mouse_up(&evt),
                        MouseEvtKind::Down(evt) => winhandle.mouse_down(&evt),
                        MouseEvtKind::Wheel(evt) => winhandle.wheel(&evt),
                        MouseEvtKind::Leave => winhandle.mouse_leave(),
                    }
                }
            }
            evt => {
                log::warn!("Unhandled pointer event: {:?}", evt);
            }
        }
    }

    fn dequeue(&self) -> Option<MouseEvtKind> {
        use wl_pointer::{Axis, ButtonState};
        // sometimes we need to ignore an event and move on
        loop {
            let event = self.queued_events.borrow_mut().pop_front()?;
            tracing::trace!("mouse event {:?}", event);
            match event {
                PointerEvent::Motion { pointer, point } => {
                    self.pos.replace(point);
                    return Some(MouseEvtKind::Move(mouse::MouseEvent {
                        pos: point,
                        buttons: *self.buttons.borrow(),
                        mods: Modifiers::empty(),
                        count: 0,
                        focus: false,
                        button: mouse::MouseButton::None,
                        wheel_delta: Vec2::ZERO,
                    }));
                }
                PointerEvent::Button { button, state } => {
                    let button = match linux_to_mouse_button(button) {
                        // Skip unsupported buttons.
                        None => {
                            tracing::debug!("unsupported button click {:?}", button);
                            continue;
                        }
                        Some(b) => b,
                    };
                    let evt = match state {
                        ButtonState::Pressed => {
                            self.buttons.borrow_mut().insert(button);
                            self.clickevent.borrow_mut().debounce(MouseEvtKind::Down(
                                mouse::MouseEvent {
                                    pos: self.pos.get(),
                                    buttons: *self.buttons.borrow(),
                                    mods: Modifiers::empty(),
                                    count: 1,
                                    focus: false,
                                    button,
                                    wheel_delta: Vec2::ZERO,
                                },
                            ))
                        }
                        ButtonState::Released => {
                            self.buttons.borrow_mut().remove(button);
                            self.clickevent.borrow_mut().debounce(MouseEvtKind::Up(
                                mouse::MouseEvent {
                                    pos: self.pos.get(),
                                    buttons: *self.buttons.borrow(),
                                    mods: Modifiers::empty(),
                                    count: 0,
                                    focus: false,
                                    button,
                                    wheel_delta: Vec2::ZERO,
                                },
                            ))
                        }
                        _ => {
                            log::error!("mouse button changed, but not pressed or released");
                            continue;
                        }
                    };
                    return Some(evt);
                }
                PointerEvent::Axis { axis, value } => {
                    let wheel_delta = match axis {
                        Axis::VerticalScroll => Vec2::new(0., value),
                        Axis::HorizontalScroll => Vec2::new(value, 0.),
                        _ => {
                            log::error!("axis direction not vertical or horizontal");
                            continue;
                        }
                    };
                    return Some(MouseEvtKind::Wheel(mouse::MouseEvent {
                        pos: self.pos.get(),
                        buttons: *self.buttons.borrow(),
                        mods: Modifiers::empty(),
                        count: 0,
                        focus: false,
                        button: mouse::MouseButton::None,
                        wheel_delta,
                    }));
                }
                PointerEvent::Leave => {
                    // The parent will remove us.
                    return Some(MouseEvtKind::Leave);
                }
            }
        }
    }
}

impl Drop for Pointer {
    fn drop(&mut self) {
        self.cursor_surface.destroy();
    }
}

#[inline]
fn linux_to_mouse_button(button: u32) -> Option<mouse::MouseButton> {
    match button {
        BTN_LEFT => Some(mouse::MouseButton::Left),
        BTN_RIGHT => Some(mouse::MouseButton::Right),
        BTN_MIDDLE => Some(mouse::MouseButton::Middle),
        _ => None,
    }
}
