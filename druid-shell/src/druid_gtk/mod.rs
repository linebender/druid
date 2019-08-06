// Copyright 2019 The xi-editor Authors.
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

//! GTK-based platform support

pub mod application;
pub mod dialog;
pub mod menu;
pub mod util;
pub mod win_main;

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::rc::Weak;
use std::sync::{Arc, Mutex};

use gdk::EventKey;
use gdk::EventMask;
use gtk::ApplicationWindow;
use gtk::Inhibit;

use piet_common::{Piet, RenderContext};

use crate::keyboard;
use crate::kurbo::{Point, Vec2};
use crate::platform::dialog::{FileDialogOptions, FileDialogType};
use crate::platform::menu::Menu;
use crate::window::{self, Cursor, MouseButton, Text, WinCtx, WinHandler};
use crate::Error;

use crate::keyboard::{KeyCode, RawKeyCode, StrOrChar};
use util::assert_main_thread;
use win_main::with_application;

#[derive(Clone, Default)]
pub struct WindowHandle {
    window: Option<ApplicationWindow>,
}

/// Builder abstraction for creating new windows
pub struct WindowBuilder {
    handler: Option<Box<WinHandler>>,
    title: String,
    cursor: Cursor,
    menu: Option<menu::Menu>,
}

#[derive(Clone)]
pub struct IdleHandle {
    queue: Option<u32>, // TODO: implement this properly
    idle_queue: Weak<Mutex<Vec<Box<IdleCallback>>>>,
}

// TODO: move this out of platform-dependent section.
trait IdleCallback: Send {
    fn call(self: Box<Self>, a: &Any);
}

impl<F: FnOnce(&Any) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &Any) {
        (*self)(a)
    }
}

struct WinCtxImpl<'a> {
    window: Option<&'a ApplicationWindow>,
    handle: &'a WindowHandle,
    text: Text<'static>,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            cursor: Cursor::Arrow,
            menu: None,
        }
    }

    pub fn set_handler(&mut self, handler: Box<WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        use gtk::{BoxExt, ContainerExt, GtkWindowExt, WidgetExt, WidgetExtManual};
        assert_main_thread();

        let handler = self
            .handler
            .expect("Tried to build a window without setting the handler");

        let handler = Arc::new(RefCell::new(handler));

        let window = with_application(|app| ApplicationWindow::new(&app));

        window.set_title(&self.title);
        // TODO(bobtwinkles): enable this when I figure out how to set the cursor on application windows
        // window.set_cursor(gdk::Cursor::new_from_nane(
        //     &window.get_display(),
        //     match self.cursor {
        //         Arrow => "default",
        //         IBeam => "text",
        //     },
        // ));

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.add(&vbox);

        let window = window;

        let handle = WindowHandle {
            window: Some(window),
        };

        if let Some(menu) = self.menu {
            let menu = menu.into_gtk_menubar(handler.clone(), &handle);
            vbox.pack_start(&menu, false, false, 0);
        }

        let drawing_area = gtk::DrawingArea::new();

        drawing_area.set_events(
            EventMask::EXPOSURE_MASK
                | EventMask::POINTER_MOTION_MASK
                | EventMask::BUTTON_PRESS_MASK
                | EventMask::BUTTON_RELEASE_MASK
                | EventMask::KEY_PRESS_MASK
                | EventMask::ENTER_NOTIFY_MASK
                | EventMask::KEY_RELEASE_MASK
                | EventMask::SCROLL_MASK,
        );

        drawing_area.set_can_focus(true);
        drawing_area.grab_focus();

        drawing_area.connect_enter_notify_event(|widget, _| {
            widget.grab_focus();

            Inhibit(true)
        });

        {
            let last_size = Cell::new((0, 0));
            let handler = Arc::clone(&handler);
            let handle = handle.clone();

            drawing_area.connect_draw(move |widget, context| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        handle: &handle,
                        window: None,
                        text: Text::new(),
                    };

                    let extents = context.clip_extents();
                    let size = (
                        (extents.2 - extents.0) as u32,
                        (extents.3 - extents.1) as u32,
                    );

                    if last_size.get() != size {
                        last_size.set(size);
                        handler.size(size.0, size.1, &mut ctx);
                    }

                    context.set_source_rgb(0.0, 0.0, 0.0);
                    context.paint();

                    context.set_source_rgb(1.0, 0.0, 0.0);
                    context.rectangle(0.0, 0.0, 100.0, 100.0);

                    context.fill();

                    // For some reason piet needs a mutable context, so give it one I guess.
                    let mut context = context.clone();
                    let mut piet_context = Piet::new(&mut context);
                    let anim = handler.paint(&mut piet_context, &mut ctx);
                    if let Err(e) = piet_context.finish() {
                        eprintln!("piet error on render: {:?}", e);
                    }

                    if anim {
                        widget.queue_draw();
                    }
                }

                Inhibit(false)
            });
        }

        {
            let handler = Arc::clone(&handler);
            let handle = handle.clone();
            drawing_area.connect_button_press_event(move |_widget, button| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        window: None, // TODO Steven
                        handle: &handle,
                        text: Text::new(),
                    };

                    let pos = Point::from(button.get_position());
                    handler.mouse_down(
                        &window::MouseEvent {
                            pos,
                            count: 1,
                            mods: gtk_modifiers_to_druid(button.get_state()),
                            button: gtk_button_to_druid(button.get_button()),
                            //ty: window::MouseType::Down,
                        },
                        &mut ctx,
                    );
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            let handle = handle.clone();
            drawing_area.connect_button_release_event(move |_widget, button| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        window: None,
                        handle: &handle,
                        text: Text::new(),
                    };

                    let pos = Point::from(button.get_position());
                    handler.mouse_up(
                        &window::MouseEvent {
                            pos,
                            mods: gtk_modifiers_to_druid(button.get_state()),
                            count: 0,
                            button: gtk_button_to_druid(button.get_button()),
                            //ty: window::MouseType::Up,
                        },
                        &mut ctx,
                    );
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            let handle = handle.clone();
            drawing_area.connect_motion_notify_event(move |_widget, motion| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        window: None,
                        handle: &handle,
                        text: Text::new(),
                    };

                    let pos = Point::from(motion.get_position());
                    let mouse_event = window::MouseEvent {
                        pos,
                        mods: gtk_modifiers_to_druid(motion.get_state()),
                        count: 0,
                        button: gtk_modifiers_to_mouse_button(motion.get_state()),
                    };

                    handler.mouse_move(&mouse_event, &mut ctx);
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            let handle = handle.clone();
            drawing_area.connect_scroll_event(move |_widget, scroll| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        window: None,
                        handle: &handle,
                        text: Text::new(),
                    };

                    use gdk::ScrollDirection;
                    let deltas = scroll.get_scroll_deltas();
                    // TODO use these deltas
                    let modifiers = gtk_modifiers_to_druid(scroll.get_state());

                    // The magic "120"s are from Microsoft's documentation for WM_MOUSEWHEEL.
                    // They claim that one "tick" on a scroll wheel should be 120 units.
                    // GTK simply reports the direction
                    match scroll.get_direction() {
                        ScrollDirection::Up => {
                            handler.wheel(Vec2::from((0.0, -120.0)), modifiers, &mut ctx);
                        }
                        ScrollDirection::Down => {
                            handler.wheel(Vec2::from((0.0, 120.0)), modifiers, &mut ctx);
                        }
                        ScrollDirection::Left => {
                            // Note: this direction is just a guess, I (bobtwinkles) don't
                            // have a way to test horizontal scroll events under GTK.
                            // If it's wrong, the right direction also needs to be changed
                            handler.wheel(Vec2::from((120.0, 0.0)), modifiers, &mut ctx);
                        }
                        ScrollDirection::Right => {
                            handler.wheel(-Vec2::from((-120.0, 0.0)), modifiers, &mut ctx);
                        }
                        ScrollDirection::Smooth => {
                            eprintln!(
                                "Warning: somehow the Druid widget got a smooth scroll event"
                            );
                        }
                        e => {
                            eprintln!(
                                "Warning: the Druid widget got some whacky scroll direction {:?}",
                                e
                            );
                        }
                    }
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            let handle = handle.clone();
            drawing_area.connect_key_press_event(move |_widget, key| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        window: None,
                        handle: &handle,
                        text: Text::new(),
                    };

                    let key_event = gtk_event_key_to_key_event(key);
                    handler.key_down(key_event, &mut ctx);
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            let handle = handle.clone();
            drawing_area.connect_key_release_event(move |_widget, key| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        window: None,
                        handle: &handle,
                        text: Text::new(),
                    };

                    let key_event = gtk_event_key_to_key_event(key);
                    handler.key_up(key_event, &mut ctx);
                }

                Inhibit(true)
            });
        }

        {
            let handler = Arc::clone(&handler);
            let handle = handle.clone();
            drawing_area.connect_destroy(move |widget| {
                if let Ok(mut handler) = handler.try_borrow_mut() {
                    let mut ctx = WinCtxImpl {
                        window: None,
                        handle: &handle,
                        text: Text::new(),
                    };
                    handler.destroy(&mut ctx);
                }
            });
        }

        vbox.pack_end(&drawing_area, true, true, 0);

        handler.borrow_mut().connect(&window::WindowHandle {
            inner: handle.clone(),
        });

        Ok(handle)
    }
}

impl WindowHandle {
    pub fn show(&self) {
        use gtk::WidgetExt;
        if let Some(window) = self.window.as_ref() {
            window.show_all();
        }
    }

    /// Close the window.
    pub fn close(&self) {
        use gtk::GtkApplicationExt;
        match self.window.as_ref() {
            Some(window) => {
                with_application(|app| {
                    app.remove_window(window);
                });
            }
            None => return,
        }
    }

    // Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        use gtk::WidgetExt;
        if let Some(window) = self.window.as_ref() {
            window.queue_draw();
        }
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        unimplemented!("WindowHandle::get_idle_handle");
    }

    /// Get the dpi of the window.
    ///
    /// TODO: we want to migrate this from dpi (with 96 as nominal) to a scale
    /// factor (with 1 as nominal).
    pub fn get_dpi(&self) -> f32 {
        // TODO: get actual dpi
        96.0
    }

    // TODO: the following methods are cut'n'paste code. A good way to DRY
    // would be to have a platform-independent trait with these as methods with
    // default implementations.

    /// Convert a dimension in px units to physical pixels (rounding).
    pub fn px_to_pixels(&self, x: f32) -> i32 {
        (x * self.get_dpi() * (1.0 / 96.0)).round() as i32
    }

    /// Convert a point in px units to physical pixels (rounding).
    pub fn px_to_pixels_xy(&self, x: f32, y: f32) -> (i32, i32) {
        let scale = self.get_dpi() * (1.0 / 96.0);
        ((x * scale).round() as i32, (y * scale).round() as i32)
    }

    /// Convert a dimension in physical pixels to px units.
    pub fn pixels_to_px<T: Into<f64>>(&self, x: T) -> f32 {
        (x.into() as f32) * 96.0 / self.get_dpi()
    }

    /// Convert a point in physical pixels to px units.
    pub fn pixels_to_px_xy<T: Into<f64>>(&self, x: T, y: T) -> (f32, f32) {
        let scale = 96.0 / self.get_dpi();
        ((x.into() as f32) * scale, (y.into() as f32) * scale)
    }

    pub fn file_dialog(
        &self,
        ty: FileDialogType,
        options: FileDialogOptions,
    ) -> Result<OsString, Error> {
        unimplemented!()
    }
}

unsafe impl Send for IdleHandle {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the main UI thread, and
    /// won't be scheduled if the associated view has been dropped.
    ///
    /// Note: the name "idle" suggests that it will be scheduled with a lower
    /// priority than other UI events, but that's not necessarily the case.
    pub fn add_idle<F>(&self, callback: F)
    where
        F: FnOnce(&Any) + Send + 'static,
    {
        if let Some(queue) = self.idle_queue.upgrade() {
            let mut queue = queue.lock().unwrap();
            if queue.is_empty() {
                unimplemented!("Idle queue wait");
            }
            queue.push(Box::new(callback));
        }
    }
}

impl<'a> WinCtx<'a> for WinCtxImpl<'a> {
    fn invalidate(&mut self) {
        self.handle.invalidate();
    }

    fn text_factory(&mut self) -> &mut Text<'a> {
        &mut self.text
    }

    fn set_cursor(&mut self, cursor: &Cursor) {
        // TODO Steven implement cursor
    }
}

/// Map a GTK mouse button to a Druid one
#[inline]
fn gtk_button_to_druid(button: u32) -> window::MouseButton {
    match button {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        4 => MouseButton::X1,
        _ => MouseButton::X2,
    }
}

/// Map the GTK modifiers into Druid bits
#[inline]
fn gtk_modifiers_to_druid(modifiers: gdk::ModifierType) -> keyboard::KeyModifiers {
    use gdk::ModifierType;

    keyboard::KeyModifiers {
        shift: modifiers.contains(ModifierType::SHIFT_MASK),
        alt: modifiers.contains(ModifierType::MOD1_MASK),
        ctrl: modifiers.contains(ModifierType::CONTROL_MASK),
        meta: modifiers.contains(ModifierType::META_MASK),
    }
}

fn gtk_modifiers_to_mouse_button(modifiers: gdk::ModifierType) -> window::MouseButton {
    use gdk::ModifierType;
    match modifiers {
        modifiers if modifiers.contains(ModifierType::BUTTON1_MASK) => MouseButton::Left,
        modifiers if modifiers.contains(ModifierType::BUTTON2_MASK) => MouseButton::Middle,
        modifiers if modifiers.contains(ModifierType::BUTTON3_MASK) => MouseButton::Right,
        modifiers if modifiers.contains(ModifierType::BUTTON4_MASK) => MouseButton::X1,
        modifiers if modifiers.contains(ModifierType::BUTTON5_MASK) => MouseButton::X2,
        _ => {
            //FIXME: what about when no modifiers match?
            MouseButton::Left
        }
    }
}

/// Map a hardware keycode to a keyval by looking up the keycode in the keymap
fn hardware_keycode_to_keyval(keycode: u16) -> Option<u32> {
    use std::ffi::c_void;
    use std::os::raw::{c_int, c_uint};
    use std::ptr;
    use std::slice;
    unsafe {
        let keymap = gdk_sys::gdk_keymap_get_default();

        let mut nkeys = 0;
        let mut keys: *mut gdk_sys::GdkKeymapKey = ptr::null_mut();
        let mut keyvals: *mut c_uint = ptr::null_mut();

        // call into gdk to retrieve the keyvals and keymap keys
        gdk_sys::gdk_keymap_get_entries_for_keycode(
            keymap,
            c_uint::from(keycode),
            &mut keys as *mut *mut gdk_sys::GdkKeymapKey,
            &mut keyvals as *mut *mut c_uint,
            &mut nkeys as *mut c_int,
        );

        if nkeys > 0 {
            let keyvals_slice = slice::from_raw_parts(keyvals, nkeys as usize);
            let keys_slice = slice::from_raw_parts(keys, nkeys as usize);

            let resolved_keyval = keys_slice.iter().enumerate().find_map(|(i, key)| {
                if key.group == 0 && key.level == 0 {
                    Some(keyvals_slice[i])
                } else {
                    None
                }
            });

            // notify glib to free the allocated arrays
            glib_sys::g_free(keyvals as *mut c_void);
            glib_sys::g_free(keys as *mut c_void);

            resolved_keyval
        } else {
            None
        }
    }
}

fn gtk_event_key_to_key_event(key: &EventKey) -> keyboard::KeyEvent {
    // the logical key being pressed
    let keyval = key.get_keyval();

    let hardware_keycode = key.get_hardware_keycode();
    let keycode = hardware_keycode_to_keyval(hardware_keycode).unwrap_or(keyval);

    // TODO how can we get the different versions from GDK?
    let text: StrOrChar = gdk::keyval_to_unicode(keyval).into();
    // TODO properly handle modifiers
    let unmodified_text: StrOrChar = gdk::keyval_to_unicode(keyval).into();

    keyboard::KeyEvent::new(
        keycode,
        false, // TODO Steven implement is_repeat
        gtk_modifiers_to_druid(key.get_state()),
        text,
        unmodified_text,
    )
}

impl From<u32> for KeyCode {
    #[allow(non_upper_case_globals)]
    fn from(raw: u32) -> KeyCode {
        use gdk::enums::key::*;
        match raw {
            a | A => KeyCode::KeyA,
            s | S => KeyCode::KeyS,
            d | D => KeyCode::KeyD,
            f | F => KeyCode::KeyF,
            h | H => KeyCode::KeyH,
            g | G => KeyCode::KeyG,
            z | Z => KeyCode::KeyZ,
            x | X => KeyCode::KeyX,
            c | C => KeyCode::KeyC,
            v | V => KeyCode::KeyV,
            b | B => KeyCode::KeyB,
            q | Q => KeyCode::KeyQ,
            w | W => KeyCode::KeyW,
            e | E => KeyCode::KeyE,
            r | R => KeyCode::KeyR,
            y | Y => KeyCode::KeyY,
            t | T => KeyCode::KeyT,
            _1 => KeyCode::Key1,
            _2 => KeyCode::Key2,
            _3 => KeyCode::Key3,
            _4 => KeyCode::Key4,
            _6 => KeyCode::Key6,
            _5 => KeyCode::Key5,
            equal => KeyCode::Equals,
            _9 => KeyCode::Key9,
            _7 => KeyCode::Key7,
            minus => KeyCode::Minus,
            _8 => KeyCode::Key8,
            _0 => KeyCode::Key0,
            bracketright => KeyCode::RightBracket,
            o | O => KeyCode::KeyO,
            u | U => KeyCode::KeyU,
            bracketleft => KeyCode::LeftBracket,
            i | I => KeyCode::KeyI,
            p | P => KeyCode::KeyP,
            Return => KeyCode::Return,
            l | L => KeyCode::KeyL,
            j | J => KeyCode::KeyJ,
            grave => KeyCode::Backtick,
            k | K => KeyCode::KeyK,
            semicolon => KeyCode::Semicolon,
            backslash => KeyCode::Backslash,
            comma => KeyCode::Comma,
            slash => KeyCode::Slash,
            n | N => KeyCode::KeyN,
            m | M => KeyCode::KeyM,
            period => KeyCode::Period,
            Tab => KeyCode::Tab,
            space => KeyCode::Space,
            BackSpace => KeyCode::Backspace,
            Escape => KeyCode::Escape,
            Caps_Lock => KeyCode::CapsLock,
            KP_Decimal => KeyCode::NumpadDecimal,
            KP_Multiply => KeyCode::NumpadMultiply,
            KP_Add => KeyCode::NumpadAdd,
            Num_Lock => KeyCode::NumLock,
            KP_Divide => KeyCode::NumpadDivide,
            KP_Enter => KeyCode::NumpadEnter,
            KP_Subtract => KeyCode::NumpadSubtract,
            KP_Equal => KeyCode::NumpadEquals,
            KP_0 => KeyCode::Numpad0,
            KP_1 => KeyCode::Numpad1,
            KP_2 => KeyCode::Numpad2,
            KP_3 => KeyCode::Numpad3,
            KP_4 => KeyCode::Numpad4,
            KP_5 => KeyCode::Numpad5,
            KP_6 => KeyCode::Numpad6,
            KP_7 => KeyCode::Numpad7,
            KP_8 => KeyCode::Numpad8,
            KP_9 => KeyCode::Numpad9,
            F5 => KeyCode::F5,
            F6 => KeyCode::F6,
            F7 => KeyCode::F7,
            F3 => KeyCode::F3,
            F8 => KeyCode::F8,
            F9 => KeyCode::F9,
            F10 => KeyCode::F10,
            F11 => KeyCode::F11,
            F12 => KeyCode::F12,
            Insert => KeyCode::Insert,
            Home => KeyCode::Home,
            Page_Up => KeyCode::PageUp,
            Delete => KeyCode::Delete,
            F4 => KeyCode::F4,
            End => KeyCode::End,
            F2 => KeyCode::F2,
            Page_Down => KeyCode::PageDown,
            F1 => KeyCode::F1,
            Left => KeyCode::ArrowLeft,
            Right => KeyCode::ArrowRight,
            Down => KeyCode::ArrowDown,
            Up => KeyCode::ArrowUp,
            quoteright => KeyCode::Quote,
            _ => {
                eprintln!("Warning: unknown keyval {}", raw);
                KeyCode::Unknown(RawKeyCode::Linux(raw))
            }
        }
    }
}
