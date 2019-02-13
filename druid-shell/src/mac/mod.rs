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

pub mod dialog;
pub mod menu;
pub mod util;
pub mod win_main;

use platform::dialog::{FileDialogOptions, FileDialogType};
use platform::menu::Menu;
use std::ffi::c_void;
use std::ffi::OsString;

use cocoa::appkit::{
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

use piet_common::Piet;

use window::{Cursor, WinHandler};
use Error;

use util::assert_main_thread;

#[derive(Clone, Default)]
pub struct WindowHandle {
    /// This is an NSView, as our concept of "window" is more the top-level container holding
    /// a view. Also, this is better for hosted applications such as VST.
    ///
    /// TODO: remove option (issue has been filed against objc, or we could manually impl default with nil)
    nsview: Option<WeakPtr>,
}

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<Box<WinHandler>>,
    title: String,
    cursor: Cursor,
}

/// This is the state associated with our custom NSView.
struct ViewState {
    handler: Box<dyn WinHandler>,
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            title: String::new(),
            cursor: Cursor::Arrow,
        }
    }

    pub fn set_handler(&mut self, handler: Box<WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        // TODO
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        assert_main_thread();
        unsafe {
            let window = NSWindow::alloc(nil)
                .initWithContentRect_styleMask_backing_defer_(
                    NSRect::new(NSPoint::new(0., 0.), NSSize::new(500., 400.)),
                    NSWindowStyleMask::NSTitledWindowMask
                        | NSWindowStyleMask::NSClosableWindowMask
                        | NSWindowStyleMask::NSMiniaturizableWindowMask
                        | NSWindowStyleMask::NSResizableWindowMask,
                    NSBackingStoreBuffered,
                    NO,
                )
                .autorelease();
            window.cascadeTopLeftFromPoint_(NSPoint::new(20.0, 20.0));
            window.setTitle_(make_nsstring(&self.title));
            window.makeKeyAndOrderFront_(nil);

            let view = make_view(self.handler.unwrap());
            let content_view = window.contentView();
            let frame = NSView::frame(content_view);
            view.initWithFrame_(frame);
            // This is to invoke the size handler; maybe do it more directly
            let () = msg_send!(view, setFrameSize: frame.size);
            content_view.addSubview_(view);

            Ok(WindowHandle {
                nsview: Some(WeakPtr::new(view)),
            })
        }
    }
}

// Wrap pointer because lazy_static requires Sync.
struct ViewClass(*const Class);
unsafe impl Sync for ViewClass {}

lazy_static! {
    static ref VIEW_CLASS: ViewClass = unsafe {
        let mut decl = ClassDecl::new("DruidView", class!(NSView)).unwrap();
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
            sel!(drawRect:),
            draw_rect as extern "C" fn(&mut Object, Sel, NSRect),
        );
        ViewClass(decl.register())
    };
}

fn make_view(handler: Box<WinHandler>) -> id {
    unsafe {
        let state = ViewState { handler };
        let state_ptr = Box::into_raw(Box::new(state));
        let view: id = msg_send![VIEW_CLASS.0, new];
        (*view).set_ivar("viewState", state_ptr as *mut c_void);
        let options: NSAutoresizingMaskOptions = NSViewWidthSizable | NSViewHeightSizable;
        view.setAutoresizingMask_(options);
        view.autorelease()
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
        println!("point: {}, {}", point.x, point.y);
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
    }
}

extern "C" fn key_down(this: &mut Object, _: Sel, nsevent: id) {
    let characters = get_characters(nsevent);
    unsafe {
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        for c in characters.chars() {
            (*view_state).handler.char(c as u32, 0);
        }
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
        let cairo_surface = QuartzSurface::create_for_cg_context(cgcontext, width, height).unwrap();
        let mut cairo_ctx = Context::new(&cairo_surface);
        cairo_ctx.set_source_rgb(0.0, 0.5, 0.0);
        cairo_ctx.paint();
        let mut piet_ctx = Piet::new(&mut cairo_ctx);
        let view_state: *mut c_void = *this.get_ivar("viewState");
        let view_state = &mut *(view_state as *mut ViewState);
        let anim = (*view_state).handler.paint(&mut piet_ctx);

        // TODO: handle animation

        let superclass = msg_send![this, superclass];
        let () = msg_send![super(this, superclass), drawRect: dirtyRect];
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

    pub fn get_dpi(&self) -> f32 {
        // TODO: get actual dpi
        96.0
    }

    pub fn file_dialog(
        &self,
        ty: FileDialogType,
        options: FileDialogOptions,
    ) -> Result<OsString, Error> {
        unimplemented!()
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
