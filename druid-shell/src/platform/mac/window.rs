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

//! macOS implementation of window creation.

#![allow(non_snake_case)]

use std::any::Any;
use std::ffi::c_void;
use std::mem;
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

use cocoa::appkit::{
    NSApp, NSApplication, NSAutoresizingMaskOptions, NSBackingStoreBuffered, NSEvent,
    NSEventModifierFlags, NSView, NSViewHeightSizable, NSViewWidthSizable, NSWindow,
    NSWindowStyleMask,
};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::declare::ClassDecl;
use objc::rc::WeakPtr;
use objc::runtime::{Class, Object, Sel};

use cairo::{Context, QuartzSurface};
use log::{error, info};

use crate::kurbo::{Point, Size, Vec2};
use crate::piet::{Piet, RenderContext};

use super::dialog;
use super::menu::Menu;
use super::util::{assert_main_thread, make_nsstring};
use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::keycodes::KeyCode;
use crate::mouse::{Cursor, MouseButton, MouseEvent};
use crate::window::{Text, TimerToken, WinCtx, WinHandler};
use crate::Error;

#[allow(non_upper_case_globals)]
const NSWindowDidBecomeKeyNotification: &str = "NSWindowDidBecomeKeyNotification";

#[derive(Clone)]
pub(crate) struct WindowHandle {
    /// This is an NSView, as our concept of "window" is more the top-level container holding
    /// a view. Also, this is better for hosted applications such as VST.
    nsview: WeakPtr,
    idle_queue: Weak<Mutex<Vec<Box<dyn IdleCallback>>>>,
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
}

#[derive(Clone)]
pub(crate) struct IdleHandle {
    nsview: WeakPtr,
    idle_queue: Weak<Mutex<Vec<Box<dyn IdleCallback>>>>,
}

/// This is the state associated with our custom NSView.
struct ViewState {
    nsview: WeakPtr,
    handler: Box<dyn WinHandler>,
    idle_queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,
    last_mods: KeyModifiers,
}

struct WinCtxImpl<'a> {
    nsview: &'a WeakPtr,
    text: Text<'static>,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(500.0, 400.0),
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
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
            let style_mask = NSWindowStyleMask::NSTitledWindowMask
                | NSWindowStyleMask::NSClosableWindowMask
                | NSWindowStyleMask::NSMiniaturizableWindowMask
                | NSWindowStyleMask::NSResizableWindowMask;
            let rect = NSRect::new(
                NSPoint::new(0., 0.),
                NSSize::new(self.size.width, self.size.height),
            );

            let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
                rect,
                style_mask,
                NSBackingStoreBuffered,
                NO,
            );

            window.cascadeTopLeftFromPoint_(NSPoint::new(20.0, 20.0));
            window.setTitle_(make_nsstring(&self.title));
            // TODO: this should probably be a tracking area instead
            window.setAcceptsMouseMovedEvents_(YES);

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
            let handle = WindowHandle {
                nsview: view_state.nsview.clone(),
                idle_queue,
            };
            (*view_state).handler.connect(&handle.clone().into());
            let mut ctx = WinCtxImpl {
                nsview: &handle.nsview,
                text: Text::new(),
            };
            (*view_state).handler.connected(&mut ctx);
            (*view_state)
                .handler
                .size(frame.size.width as u32, frame.size.height as u32, &mut ctx);

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
            sel!(mouseUp:),
            mouse_up_left as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(rightMouseUp:),
            mouse_up_right as extern "C" fn(&mut Object, Sel, id),
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
            sel!(scrollWheel:),
            scroll_wheel as extern "C" fn(&mut Object, Sel, id),
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

type BoxedCallback = Box<dyn IdleCallback>;

fn make_view(handler: Box<dyn WinHandler>) -> (id, Weak<Mutex<Vec<BoxedCallback>>>) {
    let idle_queue = Arc::new(Mutex::new(Vec::new()));
    let queue_handle = Arc::downgrade(&idle_queue);
    unsafe {
        let view: id = msg_send![VIEW_CLASS.0, new];
        let nsview = WeakPtr::new(view);
        let state = ViewState {
            nsview,
            handler,
            idle_queue,
            last_mods: KeyModifiers::default(),
        };
        let state_ptr = Box::into_raw(Box::new(state));
        (*view).set_ivar("viewState", state_ptr as *mut c_void);
        let options: NSAutoresizingMaskOptions = NSViewWidthSizable | NSViewHeightSizable;
        view.setAutoresizingMask_(options);
        (view.autorelease(), queue_handle)
    }
}

extern "C" fn set_frame_size(this: &mut Object, _: Sel, size: NSSize) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state)
            .handler
            .size(size.width as u32, size.height as u32, &mut ctx);
        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), setFrameSize: size];
    }
}

// NOTE: If we know the button (because of the origin call) we pass it through,
// otherwise we get it from the event itself.
fn mouse_event(nsevent: id, view: id, button: Option<MouseButton>) -> MouseEvent {
    unsafe {
        let button = button.unwrap_or_else(|| {
            let button = NSEvent::pressedMouseButtons(nsevent);
            get_mouse_button(button as usize)
        });
        let point = nsevent.locationInWindow();
        let view_point = view.convertPoint_fromView_(point, nil);
        let pos = Point::new(view_point.x as f64, view_point.y as f64);
        let modifiers = nsevent.modifierFlags();
        let modifiers = make_modifiers(modifiers);
        let count = nsevent.clickCount() as u32;
        MouseEvent {
            pos,
            mods: modifiers,
            count,
            button,
        }
    }
}

fn get_mouse_button(mask: usize) -> MouseButton {
    //TODO: this doesn't correctly handle multiple buttons being pressed.
    match mask {
        mask if mask & 1 > 0 => MouseButton::Left,
        mask if mask & 1 << 1 > 0 => MouseButton::Right,
        mask if mask & 1 << 2 > 0 => MouseButton::Middle,
        mask if mask & 1 << 3 > 0 => MouseButton::X1,
        mask if mask & 1 << 4 > 0 => MouseButton::X2,
        _ => {
            //FIXME: this gets called when the mouse moves, where there
            //may be no buttons down. This is mostly a problem with our API?
            MouseButton::Left
        }
    }
}

extern "C" fn mouse_down_left(this: &mut Object, _: Sel, nsevent: id) {
    mouse_down(this, nsevent, MouseButton::Left)
}

extern "C" fn mouse_down_right(this: &mut Object, _: Sel, nsevent: id) {
    mouse_down(this, nsevent, MouseButton::Right)
}

fn mouse_down(this: &mut Object, nsevent: id, button: MouseButton) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let event = mouse_event(nsevent, this as id, Some(button));
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state).handler.mouse_down(&event, &mut ctx);
    }
}

extern "C" fn mouse_up_left(this: &mut Object, _: Sel, nsevent: id) {
    mouse_up(this, nsevent, MouseButton::Left)
}

extern "C" fn mouse_up_right(this: &mut Object, _: Sel, nsevent: id) {
    mouse_up(this, nsevent, MouseButton::Right)
}

fn mouse_up(this: &mut Object, nsevent: id, button: MouseButton) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let event = mouse_event(nsevent, this as id, Some(button));
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state).handler.mouse_up(&event, &mut ctx);
    }
}

extern "C" fn mouse_move(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let event = mouse_event(nsevent, this as id, None);
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state).handler.mouse_move(&event, &mut ctx);
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
        let mods = nsevent.modifierFlags();
        let mods = make_modifiers(mods);

        let delta = Vec2::new(dx, dy);
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state).handler.wheel(delta, mods, &mut ctx);
    }
}

extern "C" fn key_down(this: &mut Object, _: Sel, nsevent: id) {
    let event = make_key_event(nsevent);

    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    let mut ctx = WinCtxImpl {
        nsview: &(*view_state).nsview,
        text: Text::new(),
    };
    (*view_state).handler.key_down(event, &mut ctx);
    view_state.last_mods = event.mods;
}

extern "C" fn key_up(this: &mut Object, _: Sel, nsevent: id) {
    let event = make_key_event(nsevent);
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    let mut ctx = WinCtxImpl {
        nsview: &(*view_state).nsview,
        text: Text::new(),
    };
    (*view_state).handler.key_up(event, &mut ctx);
    view_state.last_mods = event.mods;
}

extern "C" fn mods_changed(this: &mut Object, _: Sel, nsevent: id) {
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    let (down, event) = mods_changed_key_event(view_state.last_mods, nsevent);
    view_state.last_mods = event.mods;
    let mut ctx = WinCtxImpl {
        nsview: &(*view_state).nsview,
        text: Text::new(),
    };
    if down {
        (*view_state).handler.key_down(event, &mut ctx);
    } else {
        (*view_state).handler.key_up(event, &mut ctx);
    }
}

extern "C" fn draw_rect(this: &mut Object, _: Sel, dirtyRect: NSRect) {
    unsafe {
        let context: id = msg_send![class![NSGraphicsContext], currentContext];
        // TODO: probably should use a better type than void pointer, but it's not obvious what's best.
        // cairo_sys::CGContextRef would be better documentation-wise, but it's a type alias.
        let cgcontext: *mut c_void = msg_send![context, CGContext];
        // TODO: use width and height from view size
        let frame = NSView::frame(this as *mut _);
        let width = frame.size.width as u32;
        let height = frame.size.height as u32;
        let cairo_surface =
            QuartzSurface::create_for_cg_context(cgcontext, width, height).expect("cairo surface");
        let mut cairo_ctx = Context::new(&cairo_surface);
        cairo_ctx.set_source_rgb(0.0, 0.5, 0.0);
        cairo_ctx.paint();
        let mut piet_ctx = Piet::new(&mut cairo_ctx);
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        let anim = (*view_state).handler.paint(&mut piet_ctx, &mut ctx);
        if let Err(e) = piet_ctx.finish() {
            error!("{}", e)
        }

        if anim {
            // TODO: synchronize with screen refresh rate using CVDisplayLink instead.
            let () = msg_send!(this as *const _, performSelectorOnMainThread: sel!(redraw)
                withObject: nil waitUntilDone: NO);
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
    let handler_as_any = view_state.handler.as_any();
    for callback in queue {
        callback.call(handler_as_any);
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
    let mut ctx = WinCtxImpl {
        nsview: &(*view_state).nsview,
        text: Text::new(),
    };
    let token = unsafe {
        let user_info: id = msg_send![timer, userInfo];
        msg_send![user_info, unsignedIntValue]
    };

    (*view_state)
        .handler
        .timer(TimerToken::new(token), &mut ctx);
}

extern "C" fn handle_menu_item(this: &mut Object, _: Sel, item: id) {
    unsafe {
        let tag: isize = msg_send![item, tag];
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state).handler.command(tag as u32, &mut ctx);
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
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state).handler.got_focus(&mut ctx);
    }
}

extern "C" fn window_will_close(this: &mut Object, _: Sel, _window: id) {
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let mut ctx = WinCtxImpl {
            nsview: &(*view_state).nsview,
            text: Text::new(),
        };
        (*view_state).handler.destroy(&mut ctx);
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

    // Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        unsafe {
            // We could share impl with redraw, but we'd need to deal with nil.
            let () = msg_send![*self.nsview.load(), setNeedsDisplay: YES];
        }
    }

    /// Set the title for this menu.
    pub fn set_title(&self, title: &str) {
        unsafe {
            let window: id = msg_send![*self.nsview.load(), window];
            let title = make_nsstring(title);
            window.setTitle_(title);
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

    /// Get the dpi of the window.
    ///
    /// TODO: we want to migrate this from dpi (with 96 as nominal) to a scale
    /// factor (with 1 as nominal).
    pub fn get_dpi(&self) -> f32 {
        // TODO: get actual dpi
        96.0
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
            queue.push(Box::new(callback));
        }
    }
}

impl<'a> WinCtx<'a> for WinCtxImpl<'a> {
    fn invalidate(&mut self) {
        unsafe {
            let () = msg_send![*self.nsview.load(), setNeedsDisplay: YES];
        }
    }

    fn text_factory(&mut self) -> &mut Text<'a> {
        &mut self.text
    }

    fn set_cursor(&mut self, cursor: &Cursor) {
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

    fn request_timer(&mut self, deadline: std::time::Instant) -> TimerToken {
        let ti = time_interval_from_deadline(deadline);
        let token = next_timer_id();
        unsafe {
            let nstimer = class!(NSTimer);
            let nsnumber = class!(NSNumber);
            let user_info: id = msg_send![nsnumber, numberWithUnsignedInteger: token];
            let selector = sel!(handleTimer:);
            let view = self.nsview.load();
            let _: id = msg_send![nstimer, scheduledTimerWithTimeInterval: ti target: view selector: selector userInfo: user_info repeats: NO];
        }
        TimerToken::new(token)
    }

    fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        dialog::get_file_dialog_path(FileDialogType::Open, options)
            .map(|s| FileInfo { path: s.into() })
    }

    fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        dialog::get_file_dialog_path(FileDialogType::Save, options)
            .map(|s| FileInfo { path: s.into() })
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

fn make_key_event(event: id) -> KeyEvent {
    unsafe {
        let chars = event.characters();
        let slice = std::slice::from_raw_parts(chars.UTF8String() as *const _, chars.len());
        let text = std::str::from_utf8_unchecked(slice);

        let unmodified_chars = event.charactersIgnoringModifiers();
        let slice = std::slice::from_raw_parts(
            unmodified_chars.UTF8String() as *const _,
            unmodified_chars.len(),
        );
        let unmodified_text = std::str::from_utf8_unchecked(slice);

        let virtual_key = event.keyCode();
        let is_repeat: bool = msg_send!(event, isARepeat);
        let modifiers = event.modifierFlags();
        let modifiers = make_modifiers(modifiers);
        KeyEvent::new(virtual_key, is_repeat, modifiers, text, unmodified_text)
    }
}

fn mods_changed_key_event(prev: KeyModifiers, event: id) -> (bool, KeyEvent) {
    unsafe {
        let key_code: KeyCode = event.keyCode().into();
        let is_repeat = false;
        let modifiers = event.modifierFlags();
        let modifiers = make_modifiers(modifiers);

        let down = match key_code {
            KeyCode::LeftShift | KeyCode::RightShift if prev.shift => false,
            KeyCode::LeftAlt | KeyCode::RightAlt if prev.alt => false,
            KeyCode::LeftControl | KeyCode::RightControl if prev.ctrl => false,
            KeyCode::LeftMeta | KeyCode::RightMeta if prev.meta => false,
            _ => true,
        };
        let event = KeyEvent::new(key_code, is_repeat, modifiers, "", "");
        (down, event)
    }
}

fn make_modifiers(raw: NSEventModifierFlags) -> KeyModifiers {
    KeyModifiers {
        shift: raw.contains(NSEventModifierFlags::NSShiftKeyMask),
        alt: raw.contains(NSEventModifierFlags::NSAlternateKeyMask),
        ctrl: raw.contains(NSEventModifierFlags::NSControlKeyMask),
        meta: raw.contains(NSEventModifierFlags::NSCommandKeyMask),
    }
}

fn next_timer_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TIMER_ID: AtomicUsize = AtomicUsize::new(1);
    TIMER_ID.fetch_add(1, Ordering::Relaxed)
}
