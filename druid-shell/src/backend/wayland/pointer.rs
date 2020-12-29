use crate::{
    common_util::{ClickCounter, IdleCallback},
    dialog::{FileDialogOptions, FileDialogType, FileInfo},
    error::Error as ShellError,
    keyboard::{KbKey, KeyEvent, KeyState, Modifiers},
    kurbo::{Insets, Point, Rect, Size, Vec2},
    mouse::{Cursor, CursorDesc, MouseButton, MouseButtons, MouseEvent},
    piet::ImageFormat,
};
use std::collections::VecDeque;
use wayland_client::{
    self as wl,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_callback,
        wl_keyboard::{self, WlKeyboard},
        wl_output::WlOutput,
        wl_pointer::{self, WlPointer},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::{self, WlSurface},
    },
};
use wayland_cursor::CursorImageBuffer;
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_toplevel_decoration_v1::{
        Event as ZxdgToplevelDecorationV1Event, Mode as DecorationMode, ZxdgToplevelDecorationV1,
    },
    xdg_shell::client::{
        xdg_surface::{Event as XdgSurfaceEvent, XdgSurface},
        xdg_toplevel::{Event as XdgTopLevelEvent, XdgToplevel},
        xdg_wm_base::XdgWmBase,
    },
};

// Button constants (linux specific)
//const BTN_MOUSE: u32 = 0x110;
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
//const BTN_SIDE: u32 = 0x113;
//const BTN_EXTRA: u32 = 0x114;
//const BTN_FORWARD: u32 = 0x115;
//const BTN_BACK: u32 = 0x116;
//const BTN_TASK: u32 = 0x117;

/// Collect up mouse events then emit them together on a pointer frame.
pub(crate) struct Pointer {
    /// The wayland pointer object
    pub(crate) wl_pointer: WlPointer,
    /// Currently pressed buttons
    pub(crate) buttons: MouseButtons,
    /// Current position
    pub(crate) pos: Point,
    /// Events that have occurred since the last frame.
    pub(crate) queued_events: VecDeque<PointerEvent>,
    /// The image surface which contains the cursor image.
    pub(crate) cursor_surface: wl::Main<WlSurface>,
    /// The serial used when the `Enter` event was received
    pub(crate) enter_serial: u32,
    /// Cache the current cursor, so we can see if it changed
    pub(crate) current_cursor: Option<Cursor>,
}

/// Raw wayland pointer events.
pub(crate) enum PointerEvent {
    /// Mouse moved/entered
    Motion(Point),
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
pub(crate) enum MouseEvtKind {
    Move(MouseEvent),
    Up(MouseEvent),
    Down(MouseEvent),
    Leave,
    Wheel(MouseEvent),
}

impl Pointer {
    /// Create a new pointer
    pub fn new(cursor: wl::Main<WlSurface>, wl_pointer: WlPointer, serial: u32) -> Self {
        Pointer {
            wl_pointer,
            buttons: MouseButtons::new(),
            pos: Point::ZERO, // will get set before we emit any events
            queued_events: VecDeque::with_capacity(3), // should be enough most of the time
            cursor_surface: cursor,
            enter_serial: serial,
            current_cursor: None,
        }
    }

    #[inline]
    pub fn push(&mut self, event: PointerEvent) {
        self.queued_events.push_back(event);
    }

    #[inline]
    pub fn pop(&mut self) -> Option<PointerEvent> {
        self.queued_events.pop_front()
    }

    #[inline]
    pub fn cursor(&self) -> &WlSurface {
        &self.cursor_surface
    }
}

/// Gets any queued events.
impl Iterator for Pointer {
    type Item = MouseEvtKind;

    fn next(&mut self) -> Option<Self::Item> {
        use wl_pointer::{Axis, ButtonState};
        // sometimes we need to ignore an event and move on
        loop {
            let event = self.queued_events.pop_front()?;
            match event {
                PointerEvent::Motion(point) => {
                    self.pos = point;
                    return Some(MouseEvtKind::Move(MouseEvent {
                        pos: self.pos,
                        buttons: self.buttons,
                        // TODO
                        mods: Modifiers::empty(),
                        count: 0,
                        focus: false,
                        button: MouseButton::None,
                        wheel_delta: Vec2::ZERO,
                    }));
                }
                PointerEvent::Button { button, state } => {
                    let button = match linux_to_mouse_button(button) {
                        Some(b) => b,
                        // Skip unsupported buttons.
                        None => continue,
                    };
                    let evt = match state {
                        ButtonState::Pressed => {
                            self.buttons.insert(button);
                            MouseEvtKind::Down(MouseEvent {
                                pos: self.pos,
                                buttons: self.buttons,
                                // TODO
                                mods: Modifiers::empty(),
                                count: 1,
                                focus: false,
                                button,
                                wheel_delta: Vec2::ZERO,
                            })
                        }
                        ButtonState::Released => {
                            self.buttons.remove(button);
                            MouseEvtKind::Up(MouseEvent {
                                pos: self.pos,
                                buttons: self.buttons,
                                // TODO
                                mods: Modifiers::empty(),
                                count: 0,
                                focus: false,
                                button,
                                wheel_delta: Vec2::ZERO,
                            })
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
                    return Some(MouseEvtKind::Wheel(MouseEvent {
                        pos: self.pos,
                        buttons: self.buttons,
                        // TODO
                        mods: Modifiers::empty(),
                        count: 0,
                        focus: false,
                        button: MouseButton::None,
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
fn linux_to_mouse_button(button: u32) -> Option<MouseButton> {
    match button {
        BTN_LEFT => Some(MouseButton::Left),
        BTN_RIGHT => Some(MouseButton::Right),
        BTN_MIDDLE => Some(MouseButton::Middle),
        _ => None,
    }
}
