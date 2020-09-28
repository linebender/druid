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

//! macOS implementation of window creation.

#![allow(non_snake_case)]

use std::any::Any;
use std::ffi::c_void;
use std::mem;
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

use cocoa::appkit::{
    CGFloat, NSApp, NSApplication, NSAutoresizingMaskOptions, NSBackingStoreBuffered, NSEvent,
    NSView, NSViewHeightSizable, NSViewWidthSizable, NSWindow, NSWindowStyleMask,
};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{
    NSAutoreleasePool, NSInteger, NSPoint, NSRect, NSSize, NSString, NSUInteger,
};
use core_graphics::context::CGContextRef;
use foreign_types::ForeignTypeRef;
use lazy_static::lazy_static;
use log::{error, info};
use objc::declare::ClassDecl;
use objc::rc::WeakPtr;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::piet::{Piet, PietText, RenderContext};

use super::appkit::{
    NSRunLoopCommonModes, NSTrackingArea, NSTrackingAreaOptions, NSView as NSViewExt,
};
use super::application::Application;
use super::dialog;
use super::keyboard::{make_modifiers, KeyboardState};
use super::menu::Menu;
use super::util::{assert_main_thread, make_nsstring};
use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::keyboard_types::KeyState;
use crate::mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
use crate::region::Region;
use crate::scale::Scale;
use crate::window::{IdleToken, TimerToken, WinHandler, WindowLevel, WindowState};
use crate::Error;

#[allow(non_upper_case_globals)]
const NSWindowDidBecomeKeyNotification: &str = "NSWindowDidBecomeKeyNotification";

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
mod levels {
    use crate::window::WindowLevel;

    // These are the levels that AppKit seems to have.
    pub const NSModalPanelLevel: i32 = 24;
    pub const NSNormalWindowLevel: i32 = 0;
    pub const NSFloatingWindowLevel: i32 = 3;
    pub const NSTornOffMenuWindowLevel: i32 = NSFloatingWindowLevel;
    pub const NSSubmenuWindowLevel: i32 = NSFloatingWindowLevel;
    pub const NSModalPanelWindowLevel: i32 = 8;
    pub const NSStatusWindowLevel: i32 = 25;
    pub const NSPopUpMenuWindowLevel: i32 = 101;
    pub const NSScreenSaverWindowLevel: i32 = 1000;

    pub fn as_raw_window_level(window_level: WindowLevel) -> i32 {
        use WindowLevel::*;
        match window_level {
            AppWindow => NSNormalWindowLevel,
            Tooltip => NSFloatingWindowLevel,
            DropDown => NSFloatingWindowLevel,
            Modal => NSModalPanelWindowLevel,
        }
    }
}

#[derive(Clone)]
pub(crate) struct WindowHandle {
    /// This is an NSView, as our concept of "window" is more the top-level container holding
    /// a view. Also, this is better for hosted applications such as VST.
    nsview: WeakPtr,
    idle_queue: Weak<Mutex<Vec<IdleKind>>>,
}

impl Default for WindowHandle {
    fn default() -> Self {
        WindowHandle {
            nsview: unsafe { WeakPtr::new(nil) },
            idle_queue: Default::default(),
        }
    }
}

/// Builder abstraction for creating new windows.
pub(crate) struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    size: Size,
    min_size: Option<Size>,
    position: Option<Point>,
    level: Option<WindowLevel>,
    window_state: Option<WindowState>,
    resizable: bool,
    show_titlebar: bool,
}

#[derive(Clone)]
pub(crate) struct IdleHandle {
    nsview: WeakPtr,
    idle_queue: Weak<Mutex<Vec<IdleKind>>>,
}

/// This represents different Idle Callback Mechanism
enum IdleKind {
    Callback(Box<dyn IdleCallback>),
    Token(IdleToken),
}

/// This is the state associated with our custom NSView.
struct ViewState {
    nsview: WeakPtr,
    handler: Box<dyn WinHandler>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    /// Tracks window focusing left clicks
    focus_click: bool,
    // Tracks whether we have already received the mouseExited event
    mouse_left: bool,
    keyboard_state: KeyboardState,
    text: PietText,
}

impl WindowBuilder {
    pub fn new(_app: Application) -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(500.0, 400.0),
            min_size: None,
            position: None,
            level: None,
            window_state: None,
            resizable: true,
            show_titlebar: true,
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_min_size(&mut self, size: Size) {
        self.min_size = Some(size);
    }

    pub fn resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    pub fn show_titlebar(&mut self, show_titlebar: bool) {
        self.show_titlebar = show_titlebar;
    }

    pub fn set_level(&mut self, level: WindowLevel) {
        self.level = Some(level);
    }

    pub fn set_position(&mut self, position: Point) {
        self.position = Some(position)
    }

    pub fn set_window_state(&mut self, state: WindowState) {
        self.window_state = Some(state);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        assert_main_thread();
        unsafe {
            let mut style_mask = NSWindowStyleMask::NSClosableWindowMask
                | NSWindowStyleMask::NSMiniaturizableWindowMask;

            if self.show_titlebar {
                style_mask |= NSWindowStyleMask::NSTitledWindowMask;
            }

            if self.resizable {
                style_mask |= NSWindowStyleMask::NSResizableWindowMask;
            }

            let rect = NSRect::new(
                NSPoint::new(0., 0.),
                NSSize::new(self.size.width, self.size.height),
            );

            let window: id = msg_send![WINDOW_CLASS.0, alloc];
            let window = window.initWithContentRect_styleMask_backing_defer_(
                rect,
                style_mask,
                NSBackingStoreBuffered,
                NO,
            );

            if let Some(min_size) = self.min_size {
                let size = NSSize::new(min_size.width, min_size.height);
                window.setContentMinSize_(size);
            }

            window.cascadeTopLeftFromPoint_(NSPoint::new(20.0, 20.0));
            window.setTitle_(make_nsstring(&self.title));

            let (view, idle_queue) = make_view(self.handler.expect("view"));
            let content_view = window.contentView();
            let frame = NSView::frame(content_view);
            view.initWithFrame_(frame);

            let () = msg_send![window, setDelegate: view];

            if let Some(menu) = self.menu {
                NSApp().setMainMenu_(menu.menu);
            }

            content_view.addSubview_(view);
            let view_state: *mut c_void = *(*view).get_ivar("viewState");
            let view_state = &mut *(view_state as *mut ViewState);
            let mut handle = WindowHandle {
                nsview: view_state.nsview.clone(),
                idle_queue,
            };

            if let Some(pos) = self.position {
                handle.set_position(pos);
            }
            if let Some(window_state) = self.window_state {
                handle.set_window_state(window_state);
            }

            if let Some(level) = self.level {
                handle.set_level(level)
            }

            (*view_state).handler.connect(&handle.clone().into());
            (*view_state).handler.scale(Scale::default());
            (*view_state)
                .handler
                .size(Size::new(frame.size.width, frame.size.height));

            Ok(handle)
        }
    }
}

// Wrap pointer because lazy_static requires Sync.
struct ViewClass(*const Class);
unsafe impl Sync for ViewClass {}

lazy_static! {
    static ref VIEW_CLASS: ViewClass = unsafe {
        let mut decl = ClassDecl::new("DruidView", class!(NSView)).expect("View class defined");
        decl.add_ivar::<*mut c_void>("viewState");

        decl.add_method(
            sel!(isFlipped),
            isFlipped as extern "C" fn(&Object, Sel) -> BOOL,
        );
        extern "C" fn isFlipped(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }
        decl.add_method(
            sel!(acceptsFirstResponder),
            acceptsFirstResponder as extern "C" fn(&Object, Sel) -> BOOL,
        );
        extern "C" fn acceptsFirstResponder(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }
        // acceptsFirstMouse is called when a left mouse click would focus the window
        decl.add_method(
            sel!(acceptsFirstMouse:),
            acceptsFirstMouse as extern "C" fn(&Object, Sel, id) -> BOOL,
        );
        extern "C" fn acceptsFirstMouse(this: &Object, _sel: Sel, _nsevent: id) -> BOOL {
            unsafe {
                let view_state: *mut c_void = *this.get_ivar("viewState");
                let view_state = &mut *(view_state as *mut ViewState);
                view_state.focus_click = true;
            }
            YES
        }
        decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
        extern "C" fn dealloc(this: &Object, _sel: Sel) {
            info!("view is dealloc'ed");
            unsafe {
                let view_state: *mut c_void = *this.get_ivar("viewState");
                Box::from_raw(view_state as *mut ViewState);
            }
        }

        decl.add_method(
            sel!(windowDidBecomeKey:),
            window_did_become_key as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setFrameSize:),
            set_frame_size as extern "C" fn(&mut Object, Sel, NSSize),
        );
        decl.add_method(
            sel!(mouseDown:),
            mouse_down_left as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseDown:),
            mouse_down_right as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseDown:),
            mouse_down_other as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseUp:),
            mouse_up_left as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseUp:),
            mouse_up_right as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseUp:),
            mouse_up_other as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseMoved:),
            mouse_move as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseDragged:),
            mouse_move as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(otherMouseDragged:),
            mouse_move as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseEntered:),
            mouse_enter as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseExited:),
            mouse_leave as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(scrollWheel:),
            scroll_wheel as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(magnifyWithEvent:),
            pinch_event as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(keyDown:),
            key_down as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(sel!(keyUp:), key_up as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(
            sel!(flagsChanged:),
            mods_changed as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(drawRect:),
            draw_rect as extern "C" fn(&mut Object, Sel, NSRect),
        );
        decl.add_method(sel!(runIdle), run_idle as extern "C" fn(&mut Object, Sel));
        decl.add_method(sel!(viewWillDraw), view_will_draw as extern "C" fn(&mut Object, Sel));
        decl.add_method(sel!(redraw), redraw as extern "C" fn(&mut Object, Sel));
        decl.add_method(
            sel!(handleTimer:),
            handle_timer as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(handleMenuItem:),
            handle_menu_item as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(showContextMenu:),
            show_context_menu as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(windowWillClose:),
            window_will_close as extern "C" fn(&mut Object, Sel, id),
        );
        ViewClass(decl.register())
    };
}

fn make_view(handler: Box<dyn WinHandler>) -> (id, Weak<Mutex<Vec<IdleKind>>>) {
    let idle_queue = Arc::new(Mutex::new(Vec::new()));
    let queue_handle = Arc::downgrade(&idle_queue);
    unsafe {
        let view: id = msg_send![VIEW_CLASS.0, new];
        let nsview = WeakPtr::new(view);
        let keyboard_state = KeyboardState::new();
        let state = ViewState {
            nsview,
            handler,
            idle_queue,
            focus_click: false,
            mouse_left: true,
            keyboard_state,
            text: PietText::new_with_unique_state(),
        };
        let state_ptr = Box::into_raw(Box::new(state));
        (*view).set_ivar("viewState", state_ptr as *mut c_void);
        let options: NSAutoresizingMaskOptions = NSViewWidthSizable | NSViewHeightSizable;
        view.setAutoresizingMask_(options);

        // The rect of the tracking area doesn't matter, because
        // we use the InVisibleRect option where the OS syncs the size automatically.
        let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(0., 0.));
        let opts = NSTrackingAreaOptions::MouseEnteredAndExited
            | NSTrackingAreaOptions::MouseMoved
            | NSTrackingAreaOptions::ActiveAlways
            | NSTrackingAreaOptions::InVisibleRect;
        let tracking_area = NSTrackingArea::alloc(nil)
            .initWithRect_options_owner_userInfo(rect, opts, view, nil)
            .autorelease();
        view.addTrackingArea(tracking_area);

        (view.autorelease(), queue_handle)
    }
}

struct WindowClass(*const Class);
unsafe impl Sync for WindowClass {}

lazy_static! {
    static ref WINDOW_CLASS: WindowClass = unsafe {
        let mut decl =
            ClassDecl::new("DruidWindow", class!(NSWindow)).expect("Window class defined");
        decl.add_method(
            sel!(canBecomeKeyWindow),
            canBecomeKeyWindow as extern "C" fn(&Object, Sel) -> BOOL,
        );
        extern "C" fn canBecomeKeyWindow(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }
        WindowClass(decl.register())
    };
}

extern "C" fn set_frame_size(this: &mut Object, _: Sel, size: NSSize) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        (*view_state)
            .handler
            .size(Size::new(size.width, size.height));
        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), setFrameSize: size];
    }
}

fn mouse_event(
    nsevent: id,
    view: id,
    count: u8,
    focus: bool,
    button: MouseButton,
    wheel_delta: Vec2,
) -> MouseEvent {
    unsafe {
        let point = nsevent.locationInWindow();
        let view_point = view.convertPoint_fromView_(point, nil);
        let pos = Point::new(view_point.x as f64, view_point.y as f64);
        let buttons = get_mouse_buttons(NSEvent::pressedMouseButtons(nsevent));
        let modifiers = make_modifiers(nsevent.modifierFlags());
        MouseEvent {
            pos,
            buttons,
            mods: modifiers,
            count,
            focus,
            button,
            wheel_delta,
        }
    }
}

fn get_mouse_button(button: NSInteger) -> Option<MouseButton> {
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Right),
        2 => Some(MouseButton::Middle),
        3 => Some(MouseButton::X1),
        4 => Some(MouseButton::X2),
        _ => None,
    }
}

fn get_mouse_buttons(mask: NSUInteger) -> MouseButtons {
    let mut buttons = MouseButtons::new();
    if mask & 1 != 0 {
        buttons.insert(MouseButton::Left);
    }
    if mask & 1 << 1 != 0 {
        buttons.insert(MouseButton::Right);
    }
    if mask & 1 << 2 != 0 {
        buttons.insert(MouseButton::Middle);
    }
    if mask & 1 << 3 != 0 {
        buttons.insert(MouseButton::X1);
    }
    if mask & 1 << 4 != 0 {
        buttons.insert(MouseButton::X2);
    }
    buttons
}

extern "C" fn mouse_down_left(this: &mut Object, _: Sel, nsevent: id) {
    mouse_down(this, nsevent, MouseButton::Left);
}

extern "C" fn mouse_down_right(this: &mut Object, _: Sel, nsevent: id) {
    mouse_down(this, nsevent, MouseButton::Right);
}

extern "C" fn mouse_down_other(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        if let Some(button) = get_mouse_button(nsevent.buttonNumber()) {
            mouse_down(this, nsevent, button);
        }
    }
}

fn mouse_down(this: &mut Object, nsevent: id, button: MouseButton) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let count = nsevent.clickCount() as u8;
        let focus = view_state.focus_click && button == MouseButton::Left;
        let event = mouse_event(nsevent, this as id, count, focus, button, Vec2::ZERO);
        (*view_state).handler.mouse_down(&event);
    }
}

extern "C" fn mouse_up_left(this: &mut Object, _: Sel, nsevent: id) {
    mouse_up(this, nsevent, MouseButton::Left);
}

extern "C" fn mouse_up_right(this: &mut Object, _: Sel, nsevent: id) {
    mouse_up(this, nsevent, MouseButton::Right);
}

extern "C" fn mouse_up_other(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        if let Some(button) = get_mouse_button(nsevent.buttonNumber()) {
            mouse_up(this, nsevent, button);
        }
    }
}

fn mouse_up(this: &mut Object, nsevent: id, button: MouseButton) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let focus = if view_state.focus_click && button == MouseButton::Left {
            view_state.focus_click = false;
            true
        } else {
            false
        };
        let event = mouse_event(nsevent, this as id, 0, focus, button, Vec2::ZERO);
        (*view_state).handler.mouse_up(&event);
        // If we have already received a mouseExited event then that means
        // we're still receiving mouse events because some buttons are being held down.
        // When the last held button is released and we haven't received a mouseEntered event,
        // then we will no longer receive mouse events until the next mouseEntered event
        // and need to inform the handler of the mouse leaving.
        if view_state.mouse_left && event.buttons.is_empty() {
            (*view_state).handler.mouse_leave();
        }
    }
}

extern "C" fn mouse_move(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let event = mouse_event(nsevent, this as id, 0, false, MouseButton::None, Vec2::ZERO);
        (*view_state).handler.mouse_move(&event);
    }
}

extern "C" fn mouse_enter(this: &mut Object, _sel: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        view_state.mouse_left = false;
        let event = mouse_event(nsevent, this, 0, false, MouseButton::None, Vec2::ZERO);
        (*view_state).handler.mouse_move(&event);
    }
}

extern "C" fn mouse_leave(this: &mut Object, _: Sel, _nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        view_state.mouse_left = true;
        (*view_state).handler.mouse_leave();
    }
}

extern "C" fn scroll_wheel(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let (dx, dy) = {
            let dx = -nsevent.scrollingDeltaX() as f64;
            let dy = -nsevent.scrollingDeltaY() as f64;
            if nsevent.hasPreciseScrollingDeltas() == cocoa::base::YES {
                (dx, dy)
            } else {
                (dx * 32.0, dy * 32.0)
            }
        };

        let event = mouse_event(
            nsevent,
            this as id,
            0,
            false,
            MouseButton::None,
            Vec2::new(dx, dy),
        );
        (*view_state).handler.wheel(&event);
    }
}

extern "C" fn pinch_event(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);

        let delta: CGFloat = msg_send![nsevent, magnification];
        (*view_state).handler.zoom(delta as f64);
    }
}

extern "C" fn key_down(this: &mut Object, _: Sel, nsevent: id) {
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    if let Some(event) = (*view_state).keyboard_state.process_native_event(nsevent) {
        (*view_state).handler.key_down(event);
    }
}

extern "C" fn key_up(this: &mut Object, _: Sel, nsevent: id) {
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    if let Some(event) = (*view_state).keyboard_state.process_native_event(nsevent) {
        (*view_state).handler.key_up(event);
    }
}

extern "C" fn mods_changed(this: &mut Object, _: Sel, nsevent: id) {
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    if let Some(event) = (*view_state).keyboard_state.process_native_event(nsevent) {
        if event.state == KeyState::Down {
            (*view_state).handler.key_down(event);
        } else {
            (*view_state).handler.key_up(event);
        }
    }
}

extern "C" fn view_will_draw(this: &mut Object, _: Sel) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        (*view_state).handler.prepare_paint();
    }
}

extern "C" fn draw_rect(this: &mut Object, _: Sel, dirtyRect: NSRect) {
    unsafe {
        let context: id = msg_send![class![NSGraphicsContext], currentContext];
        //FIXME: when core_graphics is at 0.20, we should be able to use
        //core_graphics::sys::CGContextRef as our pointer type.
        let cgcontext_ptr: *mut <CGContextRef as ForeignTypeRef>::CType =
            msg_send![context, CGContext];
        let cgcontext_ref = CGContextRef::from_ptr_mut(cgcontext_ptr);

        // FIXME: use the actual invalid region instead of just this bounding box.
        // https://developer.apple.com/documentation/appkit/nsview/1483772-getrectsbeingdrawn?language=objc
        let rect = Rect::from_origin_size(
            (dirtyRect.origin.x, dirtyRect.origin.y),
            (dirtyRect.size.width, dirtyRect.size.height),
        );
        let invalid = Region::from(rect);

        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let mut piet_ctx = Piet::new_y_down(cgcontext_ref, Some(view_state.text.clone()));

        (*view_state).handler.paint(&mut piet_ctx, &invalid);
        if let Err(e) = piet_ctx.finish() {
            error!("{}", e)
        }

        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), drawRect: dirtyRect];
    }
}

extern "C" fn run_idle(this: &mut Object, _: Sel) {
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    let queue: Vec<_> = mem::replace(
        &mut view_state.idle_queue.lock().expect("queue"),
        Vec::new(),
    );
    for item in queue {
        match item {
            IdleKind::Callback(it) => it.call(view_state.handler.as_any()),
            IdleKind::Token(it) => {
                view_state.handler.as_mut().idle(it);
            }
        }
    }
}

extern "C" fn redraw(this: &mut Object, _: Sel) {
    unsafe {
        let () = msg_send![this as *const _, setNeedsDisplay: YES];
    }
}

extern "C" fn handle_timer(this: &mut Object, _: Sel, timer: id) {
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    let token = unsafe {
        let user_info: id = msg_send![timer, userInfo];
        msg_send![user_info, unsignedIntValue]
    };

    (*view_state).handler.timer(TimerToken::from_raw(token));
}

extern "C" fn handle_menu_item(this: &mut Object, _: Sel, item: id) {
    unsafe {
        let tag: isize = msg_send![item, tag];
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        (*view_state).handler.command(tag as u32);
    }
}

extern "C" fn show_context_menu(this: &mut Object, _: Sel, item: id) {
    unsafe {
        let window: id = msg_send![this as *const _, window];
        let mut location: NSPoint = msg_send![window, mouseLocationOutsideOfEventStream];
        let bounds: NSRect = msg_send![this as *const _, bounds];
        location.y = bounds.size.height - location.y;
        let _: BOOL = msg_send![item, popUpMenuPositioningItem: nil atLocation: location inView: this as *const _];
    }
}

extern "C" fn window_did_become_key(this: &mut Object, _: Sel, _notification: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        (*view_state).handler.got_focus();
    }
}

extern "C" fn window_will_close(this: &mut Object, _: Sel, _window: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        (*view_state).handler.destroy();
    }
}

impl WindowHandle {
    pub fn show(&self) {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            // register our view class to be alerted when it becomes the key view.
            let notif_center_class = class!(NSNotificationCenter);
            let notif_string = NSString::alloc(nil)
                .init_str(NSWindowDidBecomeKeyNotification)
                .autorelease();
            let notif_center: id = msg_send![notif_center_class, defaultCenter];
            let () = msg_send![notif_center, addObserver:*self.nsview.load() selector: sel!(windowDidBecomeKey:) name: notif_string object: window];
            window.makeKeyAndOrderFront_(nil)
        }
    }

    /// Close the window.
    pub fn close(&self) {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let () = msg_send![window, performSelectorOnMainThread: sel!(close) withObject: nil waitUntilDone: NO];
        }
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let () = msg_send![window, performSelectorOnMainThread: sel!(makeKeyAndOrderFront:) withObject: nil waitUntilDone: NO];
        }
    }

    pub fn request_anim_frame(&self) {
        unsafe {
            // TODO: synchronize with screen refresh rate using CVDisplayLink instead.
            let () = msg_send![*self.nsview.load(), performSelectorOnMainThread: sel!(redraw)
                withObject: nil waitUntilDone: NO];
        }
    }

    // Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        unsafe {
            // We could share impl with redraw, but we'd need to deal with nil.
            let () = msg_send![*self.nsview.load(), setNeedsDisplay: YES];
        }
    }

    /// Request invalidation of one rectangle.
    pub fn invalidate_rect(&self, rect: Rect) {
        let rect = NSRect::new(
            NSPoint::new(rect.x0, rect.y0),
            NSSize::new(rect.width(), rect.height()),
        );
        unsafe {
            // We could share impl with redraw, but we'd need to deal with nil.
            let () = msg_send![*self.nsview.load(), setNeedsDisplayInRect: rect];
        }
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        unsafe {
            let nscursor = class!(NSCursor);
            let cursor: id = match cursor {
                Cursor::Arrow => msg_send![nscursor, arrowCursor],
                Cursor::IBeam => msg_send![nscursor, IBeamCursor],
                Cursor::Crosshair => msg_send![nscursor, crosshairCursor],
                Cursor::OpenHand => msg_send![nscursor, openHandCursor],
                Cursor::NotAllowed => msg_send![nscursor, operationNotAllowedCursor],
                Cursor::ResizeLeftRight => msg_send![nscursor, resizeLeftRightCursor],
                Cursor::ResizeUpDown => msg_send![nscursor, resizeUpDownCursor],
            };
            let () = msg_send![cursor, set];
        }
    }

    pub fn request_timer(&self, deadline: std::time::Instant) -> TimerToken {
        let ti = time_interval_from_deadline(deadline);
        let token = TimerToken::next();
        unsafe {
            let nstimer = class!(NSTimer);
            let nsnumber = class!(NSNumber);
            let user_info: id = msg_send![nsnumber, numberWithUnsignedInteger: token.into_raw()];
            let selector = sel!(handleTimer:);
            let view = self.nsview.load();
            let timer: id = msg_send![nstimer, timerWithTimeInterval: ti target: view selector: selector userInfo: user_info repeats: NO];
            let runloop: id = msg_send![class!(NSRunLoop), currentRunLoop];
            let () = msg_send![runloop, addTimer: timer forMode: NSRunLoopCommonModes];
        }
        token
    }

    pub fn text(&self) -> PietText {
        let view = self.nsview.load();
        unsafe {
            if let Some(view) = (*view).as_ref() {
                let state: *mut c_void = *view.get_ivar("viewState");
                (*(state as *mut ViewState)).text.clone()
            } else {
                // this codepath should only happen during tests in druid, when view is nil
                PietText::new_with_unique_state()
            }
        }
    }

    pub fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        dialog::get_file_dialog_path(FileDialogType::Open, options)
            .map(|s| FileInfo { path: s.into() })
    }

    pub fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        dialog::get_file_dialog_path(FileDialogType::Save, options)
            .map(|s| FileInfo { path: s.into() })
    }

    /// Set the title for this menu.
    pub fn set_title(&self, title: &str) {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let title = make_nsstring(title);
            window.setTitle_(title);
        }
    }

    // TODO: Implement this
    pub fn show_titlebar(&self, _show_titlebar: bool) {}

    // Need to translate mac y coords, as they start from bottom left
    pub fn set_position(&self, position: Point) {
        unsafe {
            // TODO this should be the max y in orig mac coords
            let screen_height = crate::Screen::get_display_rect().height();
            let window: id = msg_send![*self.nsview.load(), window];
            let frame: NSRect = msg_send![window, frame];

            let mut new_frame = frame;
            new_frame.origin.x = position.x;
            new_frame.origin.y = screen_height - position.y - frame.size.height; // Flip back
            let () = msg_send![window, setFrame: new_frame display: YES];
        }
    }

    pub fn get_position(&self) -> Point {
        unsafe {
            // TODO this should be the max y in orig mac coords
            let screen_height = crate::Screen::get_display_rect().height();

            let window: id = msg_send![*self.nsview.load(), window];
            let current_frame: NSRect = msg_send![window, frame];

            Point::new(
                current_frame.origin.x,
                screen_height - current_frame.origin.y - current_frame.size.height,
            )
        }
    }

    pub fn set_level(&self, level: WindowLevel) {
        unsafe {
            let level = levels::as_raw_window_level(level);
            let window: id = msg_send![*self.nsview.load(), window];
            let () = msg_send![window, setLevel: level];
        }
    }

    pub fn set_size(&self, size: Size) {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let current_frame: NSRect = msg_send![window, frame];
            let mut new_frame = current_frame;
            new_frame.size.width = size.width;
            new_frame.size.height = size.height;
            let () = msg_send![window, setFrame: new_frame display: YES];
        }
    }

    pub fn get_size(&self) -> Size {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let current_frame: NSRect = msg_send![window, frame];
            Size::new(current_frame.size.width, current_frame.size.height)
        }
    }

    pub fn get_window_state(&self) -> WindowState {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let isMin: BOOL = msg_send![window, isMiniaturized];
            if isMin != NO {
                return WindowState::MINIMIZED;
            }
            let isZoomed: BOOL = msg_send![window, isZoomed];
            if isZoomed != NO {
                return WindowState::MAXIMIZED;
            }
        }
        WindowState::RESTORED
    }

    pub fn set_window_state(&mut self, state: WindowState) {
        let cur_state = self.get_window_state();
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            match (state, cur_state) {
                (s1, s2) if s1 == s2 => (),
                (WindowState::MINIMIZED, _) => {
                    let () = msg_send![window, performMiniaturize: self];
                }
                (WindowState::MAXIMIZED, _) => {
                    let () = msg_send![window, performZoom: self];
                }
                (WindowState::RESTORED, WindowState::MAXIMIZED) => {
                    let () = msg_send![window, performZoom: self];
                }
                (WindowState::RESTORED, WindowState::MINIMIZED) => {
                    let () = msg_send![window, deminiaturize: self];
                }
                (WindowState::RESTORED, WindowState::RESTORED) => {} // Can't be reached
            }
        }
    }

    pub fn handle_titlebar(&self, _val: bool) {
        log::warn!("WindowHandle::handle_titlebar is currently unimplemented for Mac.");
    }

    pub fn resizable(&self, resizable: bool) {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let mut style_mask: NSWindowStyleMask = window.styleMask();

            if resizable {
                style_mask |= NSWindowStyleMask::NSResizableWindowMask;
            } else {
                style_mask &= !NSWindowStyleMask::NSResizableWindowMask;
            }

            window.setStyleMask_(style_mask);
        }
    }

    pub fn set_menu(&self, menu: Menu) {
        unsafe {
            NSApp().setMainMenu_(menu.menu);
        }
    }

    //FIXME: we should be using the x, y values passed by the caller, but then
    //we have to figure out some way to pass them along with this performSelector:
    //call. This isn't super hard, I'm just not up for it right now.
    pub fn show_context_menu(&self, menu: Menu, _pos: Point) {
        unsafe {
            let () = msg_send![*self.nsview.load(), performSelectorOnMainThread: sel!(showContextMenu:) withObject: menu.menu waitUntilDone: NO];
        }
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        if self.nsview.load().is_null() {
            None
        } else {
            Some(IdleHandle {
                nsview: self.nsview.clone(),
                idle_queue: self.idle_queue.clone(),
            })
        }
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, Error> {
        // TODO: Get actual Scale
        Ok(Scale::new(1.0, 1.0))
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
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        if let Some(queue) = self.idle_queue.upgrade() {
            let mut queue = queue.lock().expect("queue lock");
            if queue.is_empty() {
                unsafe {
                    let nsview = self.nsview.load();
                    // Note: the nsview might be nil here if the window has been dropped, but that's ok.
                    let () = msg_send!(*nsview, performSelectorOnMainThread: sel!(runIdle)
                        withObject: nil waitUntilDone: NO);
                }
            }
            queue.push(IdleKind::Callback(Box::new(callback)));
        }
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        if let Some(queue) = self.idle_queue.upgrade() {
            let mut queue = queue.lock().expect("queue lock");
            if queue.is_empty() {
                unsafe {
                    let nsview = self.nsview.load();
                    // Note: the nsview might be nil here if the window has been dropped, but that's ok.
                    let () = msg_send!(*nsview, performSelectorOnMainThread: sel!(runIdle)
                        withObject: nil waitUntilDone: NO);
                }
            }
            queue.push(IdleKind::Token(token));
        }
    }
}

/// Convert an `Instant` into an NSTimeInterval, i.e. a fractional number
/// of seconds from now.
///
/// This may lose some precision for multi-month durations.
fn time_interval_from_deadline(deadline: std::time::Instant) -> f64 {
    let now = Instant::now();
    if now >= deadline {
        0.0
    } else {
        let t = deadline - now;
        let secs = t.as_secs() as f64;
        let subsecs = f64::from(t.subsec_micros()) * 0.000_001;
        secs + subsecs
    }
}
