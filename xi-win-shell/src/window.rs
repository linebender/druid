// Copyright 2018 Google LLC
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

//! Creation and management of windows.

#![allow(non_snake_case)]

use std::cell::{Cell, RefCell};
use std::mem;
use std::ptr::null_mut;
use std::rc::{Rc, Weak};

use winapi::ctypes::c_int;
use winapi::shared::basetsd::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::d2d1::*;
use winapi::um::wingdi::*;
use winapi::um::winnt::*;
use winapi::um::winuser::*;

use direct2d;
use direct2d::render_target::RenderTarget;

use Error;
use menu::Menu;
use paint::{self, PaintCtx};
use util::{OptionalFunctions, ToWide};

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<MyWndProc>,
    dwStyle: DWORD,
    title: String,
    menu: Option<Menu>,
}

#[derive(Clone, Default)]
pub struct WindowHandle(Weak<WindowState>);

struct WindowState {
    hwnd: Cell<HWND>,
    wndproc: Box<WndProc>,
}

/// App behavior, supplied by the app. Many of the "window procedure"
/// messages map to calls to this trait. The methods are non-mut because
/// the window procedure can be called recursively; implementers are
/// expected to use `RefCell` or the like, but should be careful to keep
/// the lifetime of the borrow short.
pub trait WinHandler {
    /// Provide the handler with a handle to the window so that it can
    /// invalidate or make other requests.
    fn connect(&self, handle: &WindowHandle);

    /// Request the handler to paint the window contents.
    fn paint(&self, ctx: &mut PaintCtx);

    #[allow(unused_variables)]
    /// Called when a menu item is selected.
    fn command(&self, id: u32) {}

    /// Called when the window is closing. Note that this happens earlier
    /// in the sequence than drop (at WM_DESTROY, while the latter is
    /// WM_NCDESTROY).
    // Maybe rename to destroy, or hook WM_CLOSE, so terminology is consistent?
    fn close(&self) {}
}

/// Generic handler trait for the winapi window procedure entry point.
trait WndProc {
    fn connect(&self, handle: &WindowHandle);

    fn window_proc(&self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM)
        -> Option<LRESULT>;
}

// State and logic for the winapi window procedure entry point. Note that this level
// implements policies such as the use of Direct2D for painting.
struct MyWndProc {
    handler: Box<WinHandler>,
    d2d_factory: direct2d::Factory,
    render_target: RefCell<Option<RenderTarget>>,
}

impl WndProc for MyWndProc {
    fn connect(&self, handle: &WindowHandle) {
        self.handler.connect(handle);
    }

    fn window_proc(&self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM)
        -> Option<LRESULT>
    {
        println!("wndproc msg: {}", msg);
        match msg {
            WM_PAINT => unsafe {
                if self.render_target.borrow_mut().is_none() {
                    let rt = paint::create_render_target(&self.d2d_factory, hwnd);
                    *self.render_target.borrow_mut() = rt.ok();
                }
                let mut tmp = self.render_target.borrow_mut();
                let rt = tmp.as_mut().unwrap();
                rt.begin_draw();
                self.handler.paint(&mut PaintCtx {
                    d2d_factory: &self.d2d_factory,
                    render_target: rt,
                });
                let _ = rt.end_draw();
                ValidateRect(hwnd, null_mut());
                Some(0)
            },
            WM_SIZE => unsafe {
                if let Some(ref mut rt) = self.render_target.borrow_mut().as_mut() {
                    if let Some(hrt) = rt.hwnd_rt() {
                        let width = LOWORD(lparam as u32) as u32;
                        let height = HIWORD(lparam as u32) as u32;
                        hrt.Resize(&D2D1_SIZE_U { width, height });
                    }
                }
                Some(0)
            },
            WM_COMMAND => {
                self.handler.command(wparam as u32);
                Some(0)
            }
            WM_DESTROY => {
                self.handler.close();
                None
            }
            _ => None
        }
    }
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            dwStyle: WS_OVERLAPPEDWINDOW,
            title: String::new(),
            menu: None,
        }
    }

    pub fn set_handler(&mut self, handler: Box<WinHandler>) {
        self.handler = Some(MyWndProc {
            handler,
            d2d_factory: direct2d::Factory::new().unwrap(),
            render_target: RefCell::new(None),
        });
    }

    pub fn set_scroll(&mut self, hscroll: bool, vscroll: bool) {
        self.dwStyle &= !(WS_HSCROLL | WS_VSCROLL);
        if hscroll {
            self.dwStyle |= WS_HSCROLL;
        }
        if vscroll {
            self.dwStyle |= WS_VSCROLL;
        }
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu)
    }

    pub fn build(self, optional_functions: &OptionalFunctions)
        -> Result<WindowHandle, Error>
    {
        unsafe {
            // Maybe separate registration in build api? Probably only need to
            // register once even for multiple window creation.

            // TODO: probably want configurable class name.
            let class_name = "Xi Editor".to_wide();
            let icon = LoadIconW(0 as HINSTANCE, IDI_APPLICATION);
            let cursor = LoadCursorW(0 as HINSTANCE, IDC_IBEAM);
            let brush = CreateSolidBrush(0xffffff);
            let wnd = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(win_proc_dispatch),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as HINSTANCE,
                hIcon: icon,
                hCursor: cursor,
                hbrBackground: brush,
                lpszMenuName: 0 as LPCWSTR,
                lpszClassName: class_name.as_ptr(),
            };
            let class_atom = RegisterClassW(&wnd);
            if class_atom == 0 {
                return Err(Error::Null);
            }
            let window = WindowState {
                hwnd: Cell::new(0 as HWND),
                wndproc: Box::new(self.handler.unwrap()),
            };
            let win = Rc::new(window);
            let handle = WindowHandle(Rc::downgrade(&win));

            // Simple scaling based on System Dpi (96 is equivalent to 100%)
            let dpi = if let Some(func) = optional_functions.GetDpiForSystem {
                // Only supported on windows 10
                func() as f32
            } else {
                // TODO GetDpiForMonitor is supported on windows 8.1, try falling back to that here
                96.0
            };
            let width = (500.0 * (dpi/96.0)) as i32;
            let height = (400.0 * (dpi/96.0)) as i32;

            let hmenu = match self.menu {
                Some(menu) => menu.into_hmenu(),
                None => 0 as HMENU,
            };
            let hwnd = create_window(WS_EX_OVERLAPPEDWINDOW, class_name.as_ptr(),
                self.title.to_wide().as_ptr(), self.dwStyle,
                CW_USEDEFAULT, CW_USEDEFAULT, width, height, 0 as HWND, hmenu, 0 as HINSTANCE,
                win.clone());
            if hwnd.is_null() {
                return Err(Error::Null);
            }
            win.hwnd.set(hwnd);
            win.wndproc.connect(&handle);
            mem::drop(win);
            Ok(handle)
        }
    }
}

unsafe extern "system" fn win_proc_dispatch(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM)
    -> LRESULT
{
    if msg == WM_CREATE {
        let create_struct = &*(lparam as *const CREATESTRUCTW);
        let wndproc_ptr = create_struct.lpCreateParams;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, wndproc_ptr as LONG_PTR);
    }
    let window_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
    let result = {
        if window_ptr.is_null() {
            None
        } else {
            (*window_ptr).wndproc.window_proc(hwnd, msg, wparam, lparam)
        }
    };
    if msg == WM_NCDESTROY {
        if !window_ptr.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            mem::drop(Rc::from_raw(window_ptr));
        }
    }
    match result {
        Some(lresult) => lresult,
        None => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Create a window (same parameters as CreateWindowExW) with associated WndProc.
unsafe fn create_window(
        dwExStyle: DWORD, lpClassName: LPCWSTR, lpWindowName: LPCWSTR, dwStyle: DWORD, x: c_int,
        y: c_int, nWidth: c_int, nHeight: c_int, hWndParent: HWND, hMenu: HMENU,
        hInstance: HINSTANCE, wndproc: Rc<WindowState>) -> HWND
{
    CreateWindowExW(dwExStyle, lpClassName, lpWindowName, dwStyle, x, y,
        nWidth, nHeight, hWndParent, hMenu, hInstance, Rc::into_raw(wndproc) as LPVOID)
}

impl WindowHandle {
    pub fn show(&self) {
        if let Some(w) = self.0.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                ShowWindow(hwnd, SW_SHOWNORMAL);
                UpdateWindow(hwnd);
            }
        }
    }

    pub fn close(&self) {
        if let Some(w) = self.0.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                DestroyWindow(hwnd);
            }
        }
    }
}