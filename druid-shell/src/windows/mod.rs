// Copyright 2018 The xi-editor Authors.
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

pub mod application;
pub mod dcomp;
pub mod dialog;
pub mod menu;
pub mod paint;
pub mod util;
pub mod win_main;

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::mem;
use std::ptr::{null, null_mut};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use winapi::ctypes::{c_int, c_void};
use winapi::shared::basetsd::*;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;
use winapi::um::d2d1::*;
use winapi::um::unknwnbase::*;
use winapi::um::wingdi::*;
use winapi::um::winnt::*;
use winapi::um::winuser::*;
use winapi::Interface;

use direct2d;
use direct2d::math::SizeU;
use direct2d::render_target::{GenericRenderTarget, HwndRenderTarget, RenderTarget};

use piet_common::{Piet, RenderContext};

use crate::kurbo::{Point, Vec2};
use crate::menu::Menu;
use crate::util::{as_result, FromWide, ToWide, OPTIONAL_FUNCTIONS};
use crate::Error;
use dcomp::{D3D11Device, DCompositionDevice, DCompositionTarget, DCompositionVisual};
use dialog::{get_file_dialog_path, FileDialogOptions, FileDialogType};

use crate::keyboard::{KeyCode, KeyEvent, KeyModifiers};
use crate::window::{self, Cursor, MouseButton, MouseEvent, WinHandler};

extern "system" {
    pub fn DwmFlush();
}

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    dwStyle: DWORD,
    title: String,
    cursor: Cursor,
    menu: Option<Menu>,
    present_strategy: PresentStrategy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// It's very tricky to get smooth dynamics (especially resizing) and
/// good performance on Windows. This setting lets clients experiment
/// with different strategies.
pub enum PresentStrategy {
    /// Don't try to use DXGI at all, only create Hwnd render targets.
    /// Note: on Windows 7 this is the only mode available.
    Hwnd,

    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_SEQUENTIAL. In
    /// testing, it causes diagonal banding artifacts with Nvidia
    /// adapters, and incremental present doesn't work. However, it
    /// is compatible with GDI (such as menus).
    Sequential,

    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL.
    /// In testing, it seems to perform well (including allowing smooth
    /// resizing when the frame can be rendered quickly), but isn't
    /// compatible with GDI.
    Flip,

    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL
    /// but with a redirection surface for GDI compatibility. Resize is
    /// very laggy and artifacty.
    FlipRedirect,
}

#[derive(Clone, Default)]
pub struct WindowHandle(Weak<WindowState>);

/// A handle that can get used to schedule an idle handler. Note that
/// this handle is thread safe. If the handle is used after the hwnd
/// has been destroyed, probably not much will go wrong (the XI_RUN_IDLE
/// message may be sent to a stray window).
#[derive(Clone)]
pub struct IdleHandle {
    pub(crate) hwnd: HWND,
    queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,
}

trait IdleCallback: Send {
    fn call(self: Box<Self>, a: &dyn Any);
}

impl<F: FnOnce(&dyn Any) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &dyn Any) {
        (*self)(a)
    }
}

struct WindowState {
    hwnd: Cell<HWND>,
    dpi: Cell<f32>,
    wndproc: Box<dyn WndProc>,
    idle_queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,
}

/// Generic handler trait for the winapi window procedure entry point.
trait WndProc {
    fn connect(&self, handle: &WindowHandle, state: WndState);

    fn window_proc(&self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM)
        -> Option<LRESULT>;
}

// State and logic for the winapi window procedure entry point. Note that this level
// implements policies such as the use of Direct2D for painting.
struct MyWndProc {
    handler: Box<dyn WinHandler>,
    handle: RefCell<WindowHandle>,
    d2d_factory: direct2d::Factory,
    dwrite_factory: directwrite::Factory,
    state: RefCell<Option<WndState>>,
}

struct WndState {
    render_target: Option<GenericRenderTarget>,
    dcomp_state: Option<DCompState>,
    dpi: f32,
    /// The `KeyCode` of the last `WM_KEYDOWN` event. We stash this so we can
    /// include it when handling `WM_CHAR` events.
    stashed_key_code: KeyCode,
    /// The `char` of the last `WM_CHAR` event, if there has not already been
    /// a `WM_KEYUP` event.
    stashed_char: Option<char>,
    //TODO: track surrogate orphan
}

/// State for DirectComposition. This is optional because it is only supported
/// on 8.1 and up.
struct DCompState {
    swap_chain: *mut IDXGISwapChain1,
    dcomp_device: DCompositionDevice,
    dcomp_target: DCompositionTarget,
    swapchain_visual: DCompositionVisual,
    // True if in a drag-resizing gesture (at which point the swapchain is disabled)
    sizing: bool,
}

/// Message indicating there are idle tasks to run.
const XI_RUN_IDLE: UINT = WM_USER;

impl Default for PresentStrategy {
    fn default() -> PresentStrategy {
        // We probably want to change this, but we need GDI to work. Too bad about
        // the artifacty resizing.
        PresentStrategy::FlipRedirect
    }
}

/// Must only be called while handling an input message.
/// This queries the keyboard state at the time of message delivery.
fn get_mod_state() -> KeyModifiers {
    //FIXME: does not handle windows key
    unsafe {
        let mut mod_state = KeyModifiers::default();
        if GetKeyState(VK_MENU) < 0 {
            mod_state.alt = true;
        }
        if GetKeyState(VK_CONTROL) < 0 {
            mod_state.ctrl = true;
        }
        if GetKeyState(VK_SHIFT) < 0 {
            mod_state.shift = true;
        }
        mod_state
    }
}

impl MyWndProc {
    fn rebuild_render_target(&self) {
        unsafe {
            let mut state = self.state.borrow_mut();
            let s = state.as_mut().unwrap();
            let swap_chain = s.dcomp_state.as_ref().unwrap().swap_chain;
            let rt = paint::create_render_target_dxgi(&self.d2d_factory, swap_chain, s.dpi)
                .map(|rt| rt.as_generic());
            s.render_target = rt.ok();
        }
    }

    // Renders but does not present.
    fn render(&self) {
        let mut state = self.state.borrow_mut();
        let s = state.as_mut().unwrap();
        let rt = s.render_target.as_mut().unwrap();
        rt.begin_draw();
        let anim;
        {
            let mut piet_ctx = Piet::new(&self.d2d_factory, &self.dwrite_factory, rt);
            anim = self.handler.paint(&mut piet_ctx);
            if let Err(e) = piet_ctx.finish() {
                // TODO: use proper log infrastructure
                eprintln!("piet error on render: {:?}", e);
            }
        }
        // Maybe should deal with lost device here...
        let res = rt.end_draw();
        if let Err(e) = res {
            println!("EndDraw error: {:?}", e);
        }
        if anim {
            let handle = self.handle.borrow().get_idle_handle().unwrap();
            // Note: maybe add WindowHandle as arg to idle handler so we don't need this.
            let handle2 = handle.clone();
            handle.add_idle(move |_| handle2.invalidate());
        }
    }
}

impl WndProc for MyWndProc {
    fn connect(&self, handle: &WindowHandle, state: WndState) {
        *self.handle.borrow_mut() = handle.clone();
        self.handler.connect(&window::WindowHandle {
            inner: handle.clone(),
        });
        *self.state.borrow_mut() = Some(state);
    }

    fn window_proc(
        &self,
        hwnd: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {
        //println!("wndproc msg: {}", msg);
        match msg {
            WM_ERASEBKGND => Some(0),
            WM_PAINT => unsafe {
                if self
                    .state
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .render_target
                    .is_none()
                {
                    let rt = paint::create_render_target(&self.d2d_factory, hwnd)
                        .map(|rt| rt.as_generic());
                    self.state.borrow_mut().as_mut().unwrap().render_target = rt.ok();
                }
                self.render();
                let mut state = self.state.borrow_mut();
                let s = state.as_mut().unwrap();
                if let Some(ref mut ds) = s.dcomp_state {
                    if !ds.sizing {
                        (*ds.swap_chain).Present(1, 0);
                        let _ = ds.dcomp_device.commit();
                    }
                }
                ValidateRect(hwnd, null_mut());
                Some(0)
            },
            WM_ENTERSIZEMOVE => unsafe {
                if self.state.borrow().as_ref().unwrap().dcomp_state.is_some() {
                    let rt = paint::create_render_target(&self.d2d_factory, hwnd)
                        .map(|rt| rt.as_generic());
                    self.state.borrow_mut().as_mut().unwrap().render_target = rt.ok();
                    self.handler.rebuild_resources();
                    self.render();

                    let mut state = self.state.borrow_mut();
                    let s = state.as_mut().unwrap();
                    if let Some(ref mut ds) = s.dcomp_state {
                        let _ = ds.dcomp_target.clear_root();
                        let _ = ds.dcomp_device.commit();
                        ds.sizing = true;
                    }
                }
                None
            },
            WM_EXITSIZEMOVE => unsafe {
                if self.state.borrow().as_ref().unwrap().dcomp_state.is_some() {
                    let mut rect: RECT = mem::uninitialized();
                    GetClientRect(hwnd, &mut rect);
                    let width = (rect.right - rect.left) as u32;
                    let height = (rect.bottom - rect.top) as u32;
                    let res = (*self
                        .state
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .dcomp_state
                        .as_mut()
                        .unwrap()
                        .swap_chain)
                        .ResizeBuffers(2, width, height, DXGI_FORMAT_UNKNOWN, 0);
                    if SUCCEEDED(res) {
                        self.handler.rebuild_resources();
                        self.rebuild_render_target();
                        self.render();
                        let mut state = self.state.borrow_mut();
                        let s = state.as_mut().unwrap();
                        (*s.dcomp_state.as_ref().unwrap().swap_chain).Present(0, 0);
                    } else {
                        println!("ResizeBuffers failed: 0x{:x}", res);
                    }

                    // Flush to present flicker artifact (old swapchain composited)
                    // It might actually be better to create a new swapchain here.
                    DwmFlush();

                    let mut state = self.state.borrow_mut();
                    let s = state.as_mut().unwrap();
                    if let Some(ref mut ds) = s.dcomp_state {
                        let _ = ds.dcomp_target.set_root(&mut ds.swapchain_visual);
                        let _ = ds.dcomp_device.commit();
                        ds.sizing = false;
                    }
                }
                None
            },
            WM_SIZE => unsafe {
                let width = LOWORD(lparam as u32) as u32;
                let height = HIWORD(lparam as u32) as u32;
                self.handler.size(width, height);
                let use_hwnd = if let Some(ref dcomp_state) =
                    self.state.borrow().as_ref().unwrap().dcomp_state
                {
                    dcomp_state.sizing
                } else {
                    true
                };
                if use_hwnd {
                    let mut state = self.state.borrow_mut();
                    let s = state.as_mut().unwrap();
                    if let Some(ref mut rt) = s.render_target {
                        if let Some(hrt) = cast_to_hwnd(rt) {
                            let width = LOWORD(lparam as u32) as u32;
                            let height = HIWORD(lparam as u32) as u32;
                            let size = SizeU(D2D1_SIZE_U { width, height });
                            let _ = hrt.resize(size);
                        }
                    }
                    InvalidateRect(hwnd, null_mut(), FALSE);
                } else {
                    let res;
                    {
                        let mut state = self.state.borrow_mut();
                        let mut s = state.as_mut().unwrap();
                        s.render_target = None;
                        res = (*s.dcomp_state.as_mut().unwrap().swap_chain).ResizeBuffers(
                            0,
                            width,
                            height,
                            DXGI_FORMAT_UNKNOWN,
                            0,
                        );
                    }
                    if SUCCEEDED(res) {
                        self.rebuild_render_target();
                        self.render();
                        let mut state = self.state.borrow_mut();
                        let s = state.as_mut().unwrap();
                        if let Some(ref mut dcomp_state) = s.dcomp_state {
                            (*dcomp_state.swap_chain).Present(0, 0);
                            let _ = dcomp_state.dcomp_device.commit();
                        }
                        ValidateRect(hwnd, null_mut());
                    } else {
                        println!("ResizeBuffers failed: 0x{:x}", res);
                    }
                }
                Some(0)
            },
            WM_COMMAND => {
                self.handler.command(LOWORD(wparam as u32) as u32);
                Some(0)
            }
            WM_CHAR => {
                let mut state = self.state.borrow_mut();
                let mut s = state.as_mut().unwrap();
                //FIXME: this can receive lone surrogate pairs?
                let key_code = s.stashed_key_code;
                s.stashed_char = std::char::from_u32(wparam as u32);
                let text = match s.stashed_char {
                    Some(c) => c,
                    None => {
                        eprintln!("failed to convert WM_CHAR to char: {:#X}", wparam);
                        return None;
                    }
                };

                let modifiers = get_mod_state();
                let is_repeat = (lparam & 0xFFFF) > 0;
                let event = KeyEvent::new(key_code, is_repeat, modifiers, text, text);

                if self.handler.key_down(event) {
                    Some(0)
                } else {
                    None
                }
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let mut state = self.state.borrow_mut();
                let mut s = state.as_mut().unwrap();
                let key_code: KeyCode = (wparam as i32).into();
                s.stashed_key_code = key_code;
                if key_code.is_printable() {
                    //FIXME: this will fail to propogate key combinations such as alt+s
                    return None;
                }

                let modifiers = get_mod_state();
                // bits 0-15 of iparam are the repeat count:
                // https://docs.microsoft.com/en-ca/windows/desktop/inputdev/wm-keydown
                let is_repeat = (lparam & 0xFFFF) > 0;
                let event = KeyEvent::new(key_code, is_repeat, modifiers, "", "");

                if self.handler.key_down(event) {
                    Some(0)
                } else {
                    None
                }
            }
            WM_KEYUP => {
                let mut state = self.state.borrow_mut();
                let s = state.as_mut().unwrap();
                let key_code: KeyCode = (wparam as i32).into();
                let modifiers = get_mod_state();
                let is_repeat = false;
                let text = s.stashed_char.take();
                let event = KeyEvent::new(key_code, is_repeat, modifiers, text, text);
                self.handler.key_up(event);
                Some(0)
            }
            //TODO: WM_SYSCOMMAND
            WM_MOUSEWHEEL => {
                // TODO: apply mouse sensitivity based on
                // SPI_GETWHEELSCROLLLINES setting.
                let delta_y = HIWORD(wparam as u32) as i16 as f64;
                let delta = Vec2::new(0.0, -delta_y);
                let mods = get_mod_state();
                self.handler.wheel(delta, mods);
                Some(0)
            }
            WM_MOUSEHWHEEL => {
                let delta_x = HIWORD(wparam as u32) as i16 as f64;
                let delta = Vec2::new(delta_x, 0.0);
                let mods = get_mod_state();
                self.handler.wheel(delta, mods);
                Some(0)
            }
            WM_MOUSEMOVE => {
                let x = LOWORD(lparam as u32) as i16 as i32;
                let y = HIWORD(lparam as u32) as i16 as i32;
                let (px, py) = self.handle.borrow().pixels_to_px_xy(x, y);
                let pos = Point::new(px as f64, py as f64);
                let mods = get_mod_state();
                let button = match wparam {
                    w if (w & 1) > 0 => MouseButton::Left,
                    w if (w & 1 << 1) > 0 => MouseButton::Right,
                    w if (w & 1 << 5) > 0 => MouseButton::Middle,
                    w if (w & 1 << 6) > 0 => MouseButton::X1,
                    w if (w & 1 << 7) > 0 => MouseButton::X2,
                    //FIXME: I guess we probably do want `MouseButton::None`?
                    //this feels bad, but also this gets discarded in druid anyway.
                    _ => MouseButton::Left,
                };
                let event = MouseEvent {
                    pos,
                    mods,
                    button,
                    count: 0,
                };
                self.handler.mouse_move(&event);
                Some(0)
            }
            // TODO: not clear where double-click processing should happen. Currently disabled
            // because CS_DBLCLKS is not set
            WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MBUTTONDBLCLK
            | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_RBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP
            | WM_XBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONUP => {
                let button = match msg {
                    WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP => MouseButton::Left,
                    WM_MBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP => MouseButton::Middle,
                    WM_RBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP => MouseButton::Right,
                    WM_XBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONUP => match HIWORD(wparam as u32)
                    {
                        1 => MouseButton::X1,
                        2 => MouseButton::X2,
                        _ => {
                            println!("unexpected X button event");
                            return None;
                        }
                    },
                    _ => unreachable!(),
                };
                let count = match msg {
                    WM_LBUTTONDOWN | WM_MBUTTONDOWN | WM_RBUTTONDOWN | WM_XBUTTONDOWN => 1,
                    WM_LBUTTONDBLCLK | WM_MBUTTONDBLCLK | WM_RBUTTONDBLCLK | WM_XBUTTONDBLCLK => 2,
                    WM_LBUTTONUP | WM_MBUTTONUP | WM_RBUTTONUP | WM_XBUTTONUP => 0,
                    _ => unreachable!(),
                };
                let x = LOWORD(lparam as u32) as i16 as i32;
                let y = HIWORD(lparam as u32) as i16 as i32;
                let (px, py) = self.handle.borrow().pixels_to_px_xy(x, y);
                let pos = Point::new(px as f64, py as f64);
                let mods = get_mod_state();
                let event = MouseEvent {
                    pos,
                    mods,
                    button,
                    count,
                };
                if count > 0 {
                    self.handler.mouse_down(&event);
                } else {
                    self.handler.mouse_up(&event);
                }
                Some(0)
            }
            WM_DESTROY => {
                self.handler.destroy();
                None
            }
            XI_RUN_IDLE => {
                let queue = self.handle.borrow().take_idle_queue();
                let handler_as_any = self.handler.as_any();
                for callback in queue {
                    callback.call(handler_as_any);
                }
                Some(0)
            }
            _ => None,
        }
    }
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            dwStyle: WS_OVERLAPPEDWINDOW,
            title: String::new(),
            cursor: Cursor::Arrow,
            menu: None,
            present_strategy: Default::default(),
        }
    }

    /// This takes ownership, and is typically used with UiMain
    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
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

    /// Set the default cursor for the window.
    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn set_present_strategy(&mut self, present_strategy: PresentStrategy) {
        self.present_strategy = present_strategy;
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        unsafe {
            // Maybe separate registration in build api? Probably only need to
            // register once even for multiple window creation.

            // TODO: probably want configurable class name.
            let class_name = "Xi Editor".to_wide();
            let icon = LoadIconW(0 as HINSTANCE, IDI_APPLICATION);
            let cursor = LoadCursorW(0 as HINSTANCE, self.cursor.get_lpcwstr());
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

            let wndproc = MyWndProc {
                handler: self.handler.unwrap(),
                handle: Default::default(),
                d2d_factory: direct2d::Factory::new().unwrap(),
                dwrite_factory: directwrite::Factory::new().unwrap(),
                state: RefCell::new(None),
            };

            let window = WindowState {
                hwnd: Cell::new(0 as HWND),
                dpi: Cell::new(0.0),
                wndproc: Box::new(wndproc),
                idle_queue: Default::default(),
            };
            let win = Rc::new(window);
            let handle = WindowHandle(Rc::downgrade(&win));

            // Simple scaling based on System Dpi (96 is equivalent to 100%)
            let dpi = if let Some(func) = OPTIONAL_FUNCTIONS.GetDpiForSystem {
                // Only supported on windows 10
                func() as f32
            } else {
                // TODO GetDpiForMonitor is supported on windows 8.1, try falling back to that here
                // Probably GetDeviceCaps(..., LOGPIXELSX) is the best to do pre-10
                96.0
            };
            win.dpi.set(dpi);
            let width = (500.0 * (dpi / 96.0)) as i32;
            let height = (400.0 * (dpi / 96.0)) as i32;

            let hmenu = match self.menu {
                Some(menu) => menu.into_hmenu(),
                None => 0 as HMENU,
            };
            let mut dwExStyle = 0;
            if self.present_strategy == PresentStrategy::Flip {
                dwExStyle |= WS_EX_NOREDIRECTIONBITMAP;
            }
            let hwnd = create_window(
                dwExStyle,
                class_name.as_ptr(),
                self.title.to_wide().as_ptr(),
                self.dwStyle,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                0 as HWND,
                hmenu,
                0 as HINSTANCE,
                win.clone(),
            );
            if hwnd.is_null() {
                return Err(Error::Null);
            }

            let dcomp_state = create_dcomp_state(self.present_strategy, hwnd).unwrap_or_else(|e| {
                println!("Error creating swapchain, falling back to hwnd: {:?}", e);
                None
            });

            win.hwnd.set(hwnd);
            let state = WndState {
                render_target: None,
                dcomp_state,
                dpi,
                stashed_key_code: KeyCode::Unknown(0.into()),
                stashed_char: None,
            };
            win.wndproc.connect(&handle, state);
            mem::drop(win);
            Ok(handle)
        }
    }
}

/// Choose an adapter. Here the heuristic is to choose the adapter with the
/// largest video memory, which will generally be the discrete adapter. It's
/// possible that on some systems the integrated adapter might be a better
/// choice, but that probably depends on usage.
unsafe fn choose_adapter(factory: *mut IDXGIFactory2) -> *mut IDXGIAdapter {
    let mut i = 0;
    let mut best_adapter = null_mut();
    let mut best_vram = 0;
    loop {
        let mut adapter: *mut IDXGIAdapter = null_mut();
        if !SUCCEEDED((*factory).EnumAdapters(i, &mut adapter)) {
            break;
        }
        let mut desc: DXGI_ADAPTER_DESC = mem::uninitialized();
        (*adapter).GetDesc(&mut desc);
        let vram = desc.DedicatedVideoMemory;
        if i == 0 || vram > best_vram {
            best_vram = vram;
            best_adapter = adapter;
        }
        println!(
            "{:?}: desc = {:?}, vram = {}",
            adapter,
            (&mut desc.Description[0] as LPWSTR).from_wide(),
            desc.DedicatedVideoMemory
        );
        i += 1;
    }
    best_adapter
}

unsafe fn create_dcomp_state(
    present_strategy: PresentStrategy,
    hwnd: HWND,
) -> Result<Option<DCompState>, Error> {
    if present_strategy == PresentStrategy::Hwnd {
        return Ok(None);
    }
    if let Some(create_dxgi_factory2) = OPTIONAL_FUNCTIONS.CreateDXGIFactory2 {
        let mut factory: *mut IDXGIFactory2 = null_mut();
        as_result(create_dxgi_factory2(
            0,
            &IID_IDXGIFactory2,
            &mut factory as *mut *mut IDXGIFactory2 as *mut *mut c_void,
        ))?;
        println!("dxgi factory pointer = {:?}", factory);
        let adapter = choose_adapter(factory);
        println!("adapter = {:?}", adapter);

        let mut d3d11_device = D3D11Device::new_simple()?;
        let mut d2d1_device = d3d11_device.create_d2d1_device()?;
        let mut dcomp_device = d2d1_device.create_composition_device()?;
        let mut dcomp_target = dcomp_device.create_target_for_hwnd(hwnd, true)?;

        let (swap_effect, bufs) = match present_strategy {
            PresentStrategy::Hwnd => unreachable!(),
            PresentStrategy::Sequential => (DXGI_SWAP_EFFECT_SEQUENTIAL, 1),
            PresentStrategy::Flip | PresentStrategy::FlipRedirect => {
                (DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, 2)
            }
        };
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: 1024,
            Height: 768,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: FALSE,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: bufs,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: swap_effect,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            Flags: 0,
        };
        let mut swap_chain: *mut IDXGISwapChain1 = null_mut();
        let res = (*factory).CreateSwapChainForComposition(
            d3d11_device.raw_ptr() as *mut IUnknown,
            &desc,
            null_mut(),
            &mut swap_chain,
        );
        println!("swap chain res = 0x{:x}, pointer = {:?}", res, swap_chain);

        let mut swapchain_visual = dcomp_device.create_visual()?;
        swapchain_visual.set_content_raw(swap_chain as *mut IUnknown)?;
        dcomp_target.set_root(&mut swapchain_visual)?;
        Ok(Some(DCompState {
            swap_chain,
            dcomp_device,
            dcomp_target,
            swapchain_visual,
            sizing: false,
        }))
    } else {
        Ok(None)
    }
}

unsafe extern "system" fn win_proc_dispatch(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
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
    dwExStyle: DWORD,
    lpClassName: LPCWSTR,
    lpWindowName: LPCWSTR,
    dwStyle: DWORD,
    x: c_int,
    y: c_int,
    nWidth: c_int,
    nHeight: c_int,
    hWndParent: HWND,
    hMenu: HMENU,
    hInstance: HINSTANCE,
    wndproc: Rc<WindowState>,
) -> HWND {
    CreateWindowExW(
        dwExStyle,
        lpClassName,
        lpWindowName,
        dwStyle,
        x,
        y,
        nWidth,
        nHeight,
        hWndParent,
        hMenu,
        hInstance,
        Rc::into_raw(wndproc) as LPVOID,
    )
}

impl Cursor {
    fn get_lpcwstr(&self) -> LPCWSTR {
        match self {
            Cursor::Arrow => IDC_ARROW,
            Cursor::IBeam => IDC_IBEAM,
            Cursor::Crosshair => IDC_CROSS,
            Cursor::OpenHand => IDC_HAND,
            Cursor::NotAllowed => IDC_NO,
            Cursor::ResizeLeftRight => IDC_SIZEWE,
            Cursor::ResizeUpDown => IDC_SIZENS,
        }
    }
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

    pub fn invalidate(&self) {
        if let Some(w) = self.0.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                InvalidateRect(hwnd, null(), FALSE);
            }
        }
    }

    /// Set the current mouse cursor.
    pub fn set_cursor(&self, cursor: &Cursor) {
        unsafe {
            let cursor = LoadCursorW(0 as HINSTANCE, cursor.get_lpcwstr());
            SetCursor(cursor);
        }
    }

    /// Get the raw HWND handle, for uses that are not wrapped in
    /// druid_win_shell.
    pub fn get_hwnd(&self) -> Option<HWND> {
        self.0.upgrade().map(|w| w.hwnd.get())
    }

    pub fn file_dialog(
        &self,
        ty: FileDialogType,
        options: FileDialogOptions,
    ) -> Result<OsString, Error> {
        let hwnd = self.get_hwnd().ok_or(Error::Null)?;
        unsafe { get_file_dialog_path(hwnd, ty, options) }
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.0.upgrade().map(|w| IdleHandle {
            hwnd: w.hwnd.get(),
            queue: w.idle_queue.clone(),
        })
    }

    fn take_idle_queue(&self) -> Vec<Box<dyn IdleCallback>> {
        if let Some(w) = self.0.upgrade() {
            mem::replace(&mut w.idle_queue.lock().unwrap(), Vec::new())
        } else {
            Vec::new()
        }
    }

    /// Get the dpi of the window.
    pub fn get_dpi(&self) -> f32 {
        if let Some(w) = self.0.upgrade() {
            w.dpi.get()
        } else {
            96.0
        }
    }

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
}

// There is a tiny risk of things going wrong when hwnd is sent across threads.
unsafe impl Send for IdleHandle {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the window's wndproc,
    /// which means it won't be scheduled if the window is closed.
    pub fn add_idle<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            unsafe {
                PostMessageW(self.hwnd, XI_RUN_IDLE, 0, 0);
            }
        }
        queue.push(Box::new(callback));
    }

    fn invalidate(&self) {
        unsafe {
            InvalidateRect(self.hwnd, null(), FALSE);
        }
    }
}

/// Casts render target to hwnd variant.
///
/// TODO: investigate whether there's a better way to do this.
unsafe fn cast_to_hwnd(rt: &GenericRenderTarget) -> Option<HwndRenderTarget> {
    let raw_ptr = rt.clone().get_raw();
    let mut hwnd = null_mut();
    let err = (*raw_ptr).QueryInterface(&ID2D1HwndRenderTarget::uuidof(), &mut hwnd);
    if SUCCEEDED(err) {
        Some(HwndRenderTarget::from_raw(
            hwnd as *mut ID2D1HwndRenderTarget,
        ))
    } else {
        None
    }
}
