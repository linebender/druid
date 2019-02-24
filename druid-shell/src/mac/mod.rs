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

pub mod application;
pub mod dialog;
pub mod menu;
pub mod util;
pub mod win_main;

use std::any::Any;
use std::ffi::c_void;
use std::ffi::OsString;
use std::mem;
use std::sync::{Arc, Mutex, Weak};
pub use menu::Menu;
use cocoa::appkit::{
    NSApp,
    NSApplication,
    NSApplicationActivateIgnoringOtherApps, NSAutoresizingMaskOptions, NSBackingStoreBuffered,
    NSEvent, NSRunningApplication, NSView, NSViewHeightSizable, NSViewWidthSizable, NSWindow,
    NSWindowStyleMask,
};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::declare::ClassDecl;
use objc::rc::WeakPtr;
use objc::runtime::{Class, Object, Sel};

use cairo::{Context, QuartzSurface};

use piet::RenderContext;
use piet_common::Piet;

use crate::platform::{menu::Menu, dialog::{FileDialogOptions, FileDialogType}};
use crate::window::{Cursor, WinHandler};
use crate::Error;

use util::assert_main_thread;

#[derive(Clone, Default)]
pub struct WindowHandle {
    /// This is an NSView, as our concept of "window" is more the top-level container holding
    /// a view. Also, this is better for hosted applications such as VST.
    ///
    /// TODO: remove option (issue has been filed against objc, or we could manually impl default with nil)
    /// https://github.com/SSheldon/rust-objc/issues/77
    nsview: Option<WeakPtr>,
    idle_queue: Weak<Mutex<Vec<Box<IdleCallback>>>>,
}

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<Box<WinHandler>>,
    title: String,
    cursor: Cursor,
    enable_mouse_move_events: bool,
    menu: Option<Menu>,
}

#[derive(Clone)]
pub struct IdleHandle {
    nsview: WeakPtr,
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
/// This is the state associated with our custom NSView.
struct ViewState {
    handler: Box<dyn WinHandler>,
    idle_queue: Arc<Mutex<Vec<Box<IdleCallback>>>>,
}

// fn build_menu() -> Strong<NSMenu> {
//   let app_name = NSProcessInfo::process_info().process_name();
//   let quit_string = nsstring!("Quit ").string_by_appending_string(app_name);

//   let quit_button = NSMenuItem::alloc().init_with_title_action_key_equivalent(
//     &quit_string,
//     selector!("terminate:"),
//     nsstring!("q"),
//   );
//   let mut app_menu = NSMenu::new();
//   app_menu.add_item(quit_button);

//   let mut app_menu_button = NSMenuItem::new();
//   app_menu_button.set_submenu(app_menu);

//   let mut menu = NSMenu::new();
//   menu.add_item(app_menu_button);
//   return menu;
// }

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            cursor: Cursor::Arrow,
            enable_mouse_move_events: true,
            menu: Some(Menu::default()),
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
        // TODO
    }
    pub fn set_enable_mouse_move_events(&mut self, to: bool) {
        self.enable_mouse_move_events = to;
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        assert_main_thread();
        unsafe {
            let style_mask =
                  NSWindowStyleMask::NSTitledWindowMask
                | NSWindowStyleMask::NSClosableWindowMask
                | NSWindowStyleMask::NSMiniaturizableWindowMask
                | NSWindowStyleMask::NSResizableWindowMask;
            let rect = NSRect::new(NSPoint::new(0., 0.), NSSize::new(500., 400.));

            let window: id = msg_send![WINDOW_CLASS.0, alloc];
            let mouse_move = if self.enable_mouse_move_events { YES } else { NO };
            (*window).set_ivar("acceptsMouseMove", mouse_move);
            
            msg_send!(window,
                      initWithContentRect: rect
                      styleMask: style_mask
                      backing: NSBackingStoreBuffered
                      defer: NO
                      );
            window.autorelease();
            window.cascadeTopLeftFromPoint_(NSPoint::new(20.0, 20.0));
            window.setTitle_(make_nsstring(&self.title));
            window.makeKeyAndOrderFront_(nil);
            let (view, idle_queue) = make_view(self.handler.expect("view"));
            let content_view = window.contentView();
            let frame = NSView::frame(content_view);
            view.initWithFrame_(frame);
            match self.menu {
                Some(menu) => NSApp().setMainMenu_(menu.menu),
                _ => (),
            }
            // This is to invoke the size handler; maybe do it more directly
            let () = msg_send!(view, setFrameSize: frame.size);
            content_view.addSubview_(view);
            Ok(WindowHandle {
                nsview: Some(WeakPtr::new(view)),
                idle_queue,
            })
        }
    }
}

struct WindowClass(*const Class);
unsafe impl Sync for WindowClass {}

lazy_static! {
    static ref WINDOW_CLASS: WindowClass = unsafe {
        let mut decl =
            ClassDecl::new("DruidWindow", class!(NSWindow))
            .expect("Window class defined");

        decl.add_ivar::<BOOL>("acceptsMouseMove");
        decl.add_method(
            sel!(acceptsMouseMovedEvents),
            acceptsMouseMovedEvents as extern "C" fn (&Object, Sel) -> BOOL
        );
        decl.add_method(
            sel!(enableMouseMoveEvents),
            enableMouseMoveEvents as extern "C" fn (&mut Object, Sel)
        );
        decl.add_method(
            sel!(disableMouseMoveEvents),
            disableMouseMoveEvents as extern "C" fn (&mut Object, Sel)
        );
        extern "C" fn acceptsMouseMovedEvents(this: &Object, _: Sel) -> BOOL {
            unsafe { *this.get_ivar("acceptsMouseMove") }
        }
        extern "C" fn enableMouseMoveEvents(this: &mut Object, _: Sel) {
            unsafe { this.set_ivar("acceptsMouseMove", YES) }
        }

        extern "C" fn disableMouseMoveEvents(this: &mut Object, _: Sel) {
            unsafe { this.set_ivar("acceptsMouseMove", NO) }
        }
        WindowClass(decl.register())
    };
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
            eprintln!("view is dealloc'ed");
            unsafe {
                let view_state: *mut c_void = *this.get_ivar("viewState");
                Box::from_raw(view_state as *mut ViewState);
            }
        }
        decl.add_method(
            sel!(setFrameSize:),
            set_frame_size as extern "C" fn(&mut Object, Sel, NSSize),
        );
        decl.add_method(
            sel!(mouseDown:),
            mouse_down as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(keyDown:),
            key_down as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseMoved:),
            mouse_moved as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(drawRect:),
            draw_rect as extern "C" fn(&mut Object, Sel, NSRect),
        );
        decl.add_method(
            sel!(runIdle),
            run_idle as extern "C" fn(&mut Object, Sel),
        );
        decl.add_method(
            sel!(redraw),
            redraw as extern "C" fn(&mut Object, Sel),
        );
        ViewClass(decl.register())
    };
}

fn make_view(handler: Box<WinHandler>) -> (id, Weak<Mutex<Vec<Box<IdleCallback>>>>) {
    let idle_queue = Arc::new(Mutex::new(Vec::new()));
    let queue_handle = Arc::downgrade(&idle_queue);
    let state = ViewState {
        handler,
        idle_queue,
    };
    let state_ptr = Box::into_raw(Box::new(state));
    unsafe {
        let view: id = msg_send![VIEW_CLASS.0, new];
        (*view).set_ivar("viewState", state_ptr as *mut c_void);
        let options: NSAutoresizingMaskOptions = NSViewWidthSizable | NSViewHeightSizable;
        view.setAutoresizingMask_(options);
        (view.autorelease(), queue_handle)
    }
}

extern "C" fn set_frame_size(this: &mut Object, _: Sel, size: NSSize) {
    println!("size: {}x{}", size.width, size.height);
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        (*view_state)
            .handler
            .size(size.width as u32, size.height as u32);
        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), setFrameSize: size];
    }
}

extern "C" fn mouse_down(this: &mut Object, _: Sel, nsevent: id) {
    unsafe {
        let point = nsevent.locationInWindow();
        println!("mouse down, point: {}, {}", point.x, point.y);
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
    }
}

extern "C" fn mouse_moved(this: &mut Object, _: Sel, nsevent: id) {
  unsafe {
    let point = nsevent.locationInWindow();
    println!("mouse moved, point: {}, {}", point.x, point.y);
  }
}
extern "C" fn key_down(this: &mut Object, _: Sel, nsevent: id) {
    let characters = get_characters(nsevent);
    dbg!(&characters);
    let view_state = unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    for c in characters.chars() {
        (*view_state).handler.char(c as u32, 0);
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
        let cairo_surface = QuartzSurface::create_for_cg_context(cgcontext, width, height).expect("cairo surface");
        let mut cairo_ctx = Context::new(&cairo_surface);
        cairo_ctx.set_source_rgb(0.0, 0.5, 0.0);
        cairo_ctx.paint();
        let mut piet_ctx = Piet::new(&mut cairo_ctx);
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let anim = (*view_state).handler.paint(&mut piet_ctx);
        piet_ctx.finish();
        // TODO: log errors

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
    let queue: Vec<_> = mem::replace(&mut view_state.idle_queue.lock().expect("queue"), Vec::new());
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

impl WindowHandle {
    pub fn show(&self) {
        unsafe {
            let current_app = NSRunningApplication::currentApplication(nil);
            current_app.activateWithOptions_(NSApplicationActivateIgnoringOtherApps);
            // TODO: do makeKeyAndOrderFront_ here (not in window build)
        }
    }

    /// Close the window.
    pub fn close(&self) {
        if let Some(ref nsview) = self.nsview {
            unsafe {
                let window: id = msg_send![*nsview.load(), window];
                window.close();
            }
        }
    }

    // Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        if let Some(ref nsview) = self.nsview {
            unsafe {
                // We could share impl with redraw, but we'd need to deal with nil.
                let () = msg_send![*nsview.load(), setNeedsDisplay: YES];
            }
        }
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        // TODO: maybe try harder to return None if window has been dropped.
        self.nsview.as_ref().map(|nsview| IdleHandle {
            nsview: nsview.clone(),
            idle_queue: self.idle_queue.clone(),
        })
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

fn make_nsstring(s: &str) -> id {
    unsafe { NSString::alloc(nil).init_str(s) }
}

fn get_characters(event: id) -> String {
    unsafe {
        let characters = event.characters();
        let slice =
            std::slice::from_raw_parts(characters.UTF8String() as *const _, characters.len());
        std::str::from_utf8_unchecked(slice).to_owned()
    }
}
