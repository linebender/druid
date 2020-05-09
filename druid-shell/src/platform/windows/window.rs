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

#![allow(non_snake_case, clippy::cast_lossless)]

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::mem;
use std::ptr::{null, null_mut};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use log::{debug, error, warn};
use winapi::ctypes::{c_int, c_void};
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;
use winapi::um::d2d1::*;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::unknwnbase::*;
use winapi::um::winnt::*;
use winapi::um::winuser::*;

use piet_common::d2d::{D2DFactory, DeviceContext};
use piet_common::dwrite::DwriteFactory;

use crate::platform::windows::HwndRenderTarget;

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::piet::{Piet, RenderContext};

use super::accels::register_accel;
use super::application::Application;
use super::dcomp::{D3D11Device, DCompositionDevice, DCompositionTarget, DCompositionVisual};
use super::dialog::get_file_dialog_path;
use super::error::Error;
use super::menu::Menu;
use super::paint;
use super::timers::TimerSlots;
use super::util::{as_result, FromWide, ToWide, OPTIONAL_FUNCTIONS};

use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::keyboard::{KeyEvent, KeyModifiers};
use crate::keycodes::KeyCode;
use crate::mouse::{Cursor, MouseButton, MouseButtons, MouseEvent};
use crate::window::{IdleToken, Text, TimerToken, WinHandler};

extern "system" {
    pub fn DwmFlush();
}

/// Builder abstraction for creating new windows.
pub(crate) struct WindowBuilder {
    app: Application,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    present_strategy: PresentStrategy,
    resizable: bool,
    show_titlebar: bool,
    size: Size,
    min_size: Option<Size>,
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
    #[allow(dead_code)]
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

#[derive(Clone)]
pub struct WindowHandle {
    dwrite_factory: DwriteFactory,
    state: Weak<WindowState>,
}

/// A handle that can get used to schedule an idle handler. Note that
/// this handle is thread safe. If the handle is used after the hwnd
/// has been destroyed, probably not much will go wrong (the DS_RUN_IDLE
/// message may be sent to a stray window).
#[derive(Clone)]
pub struct IdleHandle {
    pub(crate) hwnd: HWND,
    queue: Arc<Mutex<Vec<IdleKind>>>,
}

/// This represents different Idle Callback Mechanism
enum IdleKind {
    Callback(Box<dyn IdleCallback>),
    Token(IdleToken),
}

/// This is the low level window state. All mutable contents are protected
/// by interior mutability, so we can handle reentrant calls.
struct WindowState {
    hwnd: Cell<HWND>,
    dpi: Cell<f32>,
    wndproc: Box<dyn WndProc>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    timers: Arc<Mutex<TimerSlots>>,
}

/// Generic handler trait for the winapi window procedure entry point.
trait WndProc {
    fn connect(&self, handle: &WindowHandle, state: WndState);

    fn cleanup(&self, hwnd: HWND);

    fn window_proc(&self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM)
        -> Option<LRESULT>;
}

// State and logic for the winapi window procedure entry point. Note that this level
// implements policies such as the use of Direct2D for painting.
struct MyWndProc {
    app: Application,
    handle: RefCell<WindowHandle>,
    d2d_factory: D2DFactory,
    dwrite_factory: DwriteFactory,
    state: RefCell<Option<WndState>>,
    present_strategy: PresentStrategy,
}

/// The mutable state of the window.
struct WndState {
    handler: Box<dyn WinHandler>,
    render_target: Option<DeviceContext>,
    dcomp_state: Option<DCompState>,
    dpi: f32,
    min_size: Option<Size>,
    /// The `KeyCode` of the last `WM_KEYDOWN` event. We stash this so we can
    /// include it when handling `WM_CHAR` events.
    stashed_key_code: KeyCode,
    /// The `char` of the last `WM_CHAR` event, if there has not already been
    /// a `WM_KEYUP` event.
    stashed_char: Option<char>,
    // Stores a set of all mouse buttons that are currently holding mouse
    // capture. When the first mouse button is down on our window we enter
    // capture, and we hold it until the last mouse button is up.
    captured_mouse_buttons: MouseButtons,
    // Is this window the topmost window under the mouse cursor
    has_mouse_focus: bool,
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
const DS_RUN_IDLE: UINT = WM_USER;

/// Message relaying a request to destroy the window.
///
/// Calling `DestroyWindow` from inside the handler is problematic
/// because it will recursively cause a `WM_DESTROY` message to be
/// sent to the window procedure, even while the handler is borrowed.
/// Thus, the message is dropped and the handler doesn't run.
///
/// As a solution, instead of immediately calling `DestroyWindow`, we
/// send this message to request destroying the window, so that at the
/// time it is handled, we can successfully borrow the handler.
pub(crate) const DS_REQUEST_DESTROY: UINT = WM_USER + 1;

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
    KeyModifiers {
        shift: get_mod_state_shift(),
        alt: get_mod_state_alt(),
        ctrl: get_mod_state_ctrl(),
        meta: get_mod_state_win(),
    }
}

#[inline]
fn get_mod_state_shift() -> bool {
    unsafe { GetKeyState(VK_SHIFT) < 0 }
}

#[inline]
fn get_mod_state_alt() -> bool {
    unsafe { GetKeyState(VK_MENU) < 0 }
}

#[inline]
fn get_mod_state_ctrl() -> bool {
    unsafe { GetKeyState(VK_CONTROL) < 0 }
}

#[inline]
fn get_mod_state_win() -> bool {
    unsafe { GetKeyState(VK_LWIN) < 0 || GetKeyState(VK_RWIN) < 0 }
}

/// Extract the buttons that are being held down from wparam in mouse events.
fn get_buttons(wparam: WPARAM) -> MouseButtons {
    let mut buttons = MouseButtons::new();
    if wparam & MK_LBUTTON != 0 {
        buttons.insert(MouseButton::Left);
    }
    if wparam & MK_RBUTTON != 0 {
        buttons.insert(MouseButton::Right);
    }
    if wparam & MK_MBUTTON != 0 {
        buttons.insert(MouseButton::Middle);
    }
    if wparam & MK_XBUTTON1 != 0 {
        buttons.insert(MouseButton::X1);
    }
    if wparam & MK_XBUTTON2 != 0 {
        buttons.insert(MouseButton::X2);
    }
    buttons
}

fn is_point_in_client_rect(hwnd: HWND, x: i32, y: i32) -> bool {
    unsafe {
        let mut client_rect = mem::MaybeUninit::uninit();
        if GetClientRect(hwnd, client_rect.as_mut_ptr()) == FALSE {
            warn!(
                "failed to get client rect: {}",
                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
            );
            return false;
        }
        let client_rect = client_rect.assume_init();
        let mouse_point = POINT { x, y };
        PtInRect(&client_rect, mouse_point) != FALSE
    }
}

impl WndState {
    fn rebuild_render_target(&mut self, d2d: &D2DFactory) {
        unsafe {
            let swap_chain = self.dcomp_state.as_ref().unwrap().swap_chain;
            let rt = paint::create_render_target_dxgi(d2d, swap_chain, self.dpi)
                .map(|rt| rt.as_device_context().expect("TODO remove this expect"));
            self.render_target = rt.ok();
        }
    }

    // Renders but does not present.
    fn render(
        &mut self,
        d2d: &D2DFactory,
        dw: &DwriteFactory,
        handle: &RefCell<WindowHandle>,
        invalid_rect: Rect,
    ) {
        let rt = self.render_target.as_mut().unwrap();
        rt.begin_draw();
        let anim;
        {
            let mut piet_ctx = Piet::new(d2d, dw, rt);
            // The documentation on DXGI_PRESENT_PARAMETERS says we "must not update any
            // pixel outside of the dirty rectangles."
            piet_ctx.clip(invalid_rect);
            anim = self.handler.paint(&mut piet_ctx, invalid_rect);
            if let Err(e) = piet_ctx.finish() {
                error!("piet error on render: {:?}", e);
            }
        }
        // Maybe should deal with lost device here...
        let res = rt.end_draw();
        if let Err(e) = res {
            error!("EndDraw error: {:?}", e);
        }
        if anim {
            let handle = handle.borrow().get_idle_handle().unwrap();
            // Note: maybe add WindowHandle as arg to idle handler so we don't need this.
            let handle2 = handle.clone();
            handle.add_idle_callback(move |_| handle2.invalidate());
        }
    }

    fn enter_mouse_capture(&mut self, hwnd: HWND, button: MouseButton) {
        if self.captured_mouse_buttons.is_empty() {
            unsafe {
                SetCapture(hwnd);
            }
        }
        self.captured_mouse_buttons.insert(button);
    }

    fn exit_mouse_capture(&mut self, button: MouseButton) -> bool {
        self.captured_mouse_buttons.remove(button);
        self.captured_mouse_buttons.is_empty()
    }
}

impl MyWndProc {
    /// Create debugging output for dropped messages due to wndproc reentrancy.
    ///
    /// In the future, we choose to do something else other than logging and dropping,
    /// such as queuing and replaying after the nested call returns.
    fn log_dropped_msg(&self, hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) {
        error!(
            "dropped message 0x{:x}, hwnd={:?}, wparam=0x{:x}, lparam=0x{:x}",
            msg, hwnd, wparam, lparam
        );
    }
}

impl WndProc for MyWndProc {
    fn connect(&self, handle: &WindowHandle, state: WndState) {
        *self.handle.borrow_mut() = handle.clone();
        *self.state.borrow_mut() = Some(state);
    }

    fn cleanup(&self, hwnd: HWND) {
        self.app.remove_window(hwnd);
    }

    #[allow(clippy::cognitive_complexity)]
    fn window_proc(
        &self,
        hwnd: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {
        //println!("wndproc msg: {}", msg);
        match msg {
            WM_CREATE => {
                let dcomp_state = unsafe {
                    create_dcomp_state(self.present_strategy, hwnd).unwrap_or_else(|e| {
                        warn!("Creating swapchain failed, falling back to hwnd: {:?}", e);
                        None
                    })
                };

                self.state.borrow_mut().as_mut().unwrap().dcomp_state = dcomp_state;
                if let Some(state) = self.handle.borrow().state.upgrade() {
                    state.hwnd.set(hwnd);
                }
                let handle = self.handle.borrow().to_owned();
                if let Some(state) = self.state.borrow_mut().as_mut() {
                    state.handler.connect(&handle.into());
                }

                Some(0)
            }
            WM_ERASEBKGND => Some(0),
            WM_SETFOCUS => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    s.handler.got_focus();
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_PAINT => unsafe {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let mut rect: RECT = mem::zeroed();
                    GetUpdateRect(hwnd, &mut rect, FALSE);
                    let s = s.as_mut().unwrap();
                    if s.render_target.is_none() {
                        let rt = paint::create_render_target(&self.d2d_factory, hwnd);
                        s.render_target = rt.ok();
                    }
                    s.handler.rebuild_resources();
                    s.render(
                        &self.d2d_factory,
                        &self.dwrite_factory,
                        &self.handle,
                        self.handle.borrow().rect_to_px(rect),
                    );
                    if let Some(ref mut ds) = s.dcomp_state {
                        let params = DXGI_PRESENT_PARAMETERS {
                            DirtyRectsCount: 1,
                            pDirtyRects: &mut rect,
                            pScrollRect: null_mut(),
                            pScrollOffset: null_mut(),
                        };
                        if !ds.sizing {
                            (*ds.swap_chain).Present1(1, 0, &params);
                            let _ = ds.dcomp_device.commit();
                        }
                    }
                    ValidateRect(hwnd, null_mut());
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            },
            WM_ENTERSIZEMOVE => unsafe {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    if s.dcomp_state.is_some() {
                        let mut rect: RECT = mem::zeroed();
                        if GetClientRect(hwnd, &mut rect) == FALSE {
                            log::warn!(
                                "GetClientRect failed: {}",
                                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                            );
                            return None;
                        }
                        let rt = paint::create_render_target(&self.d2d_factory, hwnd);
                        s.render_target = rt.ok();
                        {
                            s.handler.rebuild_resources();
                            s.render(
                                &self.d2d_factory,
                                &self.dwrite_factory,
                                &self.handle,
                                self.handle.borrow().rect_to_px(rect),
                            );
                        }

                        if let Some(ref mut ds) = s.dcomp_state {
                            let _ = ds.dcomp_target.clear_root();
                            let _ = ds.dcomp_device.commit();
                            ds.sizing = true;
                        }
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                None
            },
            WM_EXITSIZEMOVE => unsafe {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    if s.dcomp_state.is_some() {
                        let mut rect: RECT = mem::zeroed();
                        if GetClientRect(hwnd, &mut rect) == FALSE {
                            log::warn!(
                                "GetClientRect failed: {}",
                                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                            );
                            return None;
                        }
                        let width = (rect.right - rect.left) as u32;
                        let height = (rect.bottom - rect.top) as u32;
                        let res = (*s.dcomp_state.as_mut().unwrap().swap_chain).ResizeBuffers(
                            2,
                            width,
                            height,
                            DXGI_FORMAT_UNKNOWN,
                            0,
                        );
                        if SUCCEEDED(res) {
                            s.handler.rebuild_resources();
                            s.rebuild_render_target(&self.d2d_factory);
                            s.render(
                                &self.d2d_factory,
                                &self.dwrite_factory,
                                &self.handle,
                                self.handle.borrow().rect_to_px(rect),
                            );
                            (*s.dcomp_state.as_ref().unwrap().swap_chain).Present(0, 0);
                        } else {
                            error!("ResizeBuffers failed: 0x{:x}", res);
                        }

                        // Flush to present flicker artifact (old swapchain composited)
                        // It might actually be better to create a new swapchain here.
                        DwmFlush();

                        if let Some(ref mut ds) = s.dcomp_state {
                            let _ = ds.dcomp_target.set_root(&mut ds.swapchain_visual);
                            let _ = ds.dcomp_device.commit();
                            ds.sizing = false;
                        }
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                None
            },
            WM_SIZE => unsafe {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let width = LOWORD(lparam as u32) as u32;
                    let height = HIWORD(lparam as u32) as u32;
                    s.handler.size(width, height);
                    let use_hwnd = if let Some(ref dcomp_state) = s.dcomp_state {
                        dcomp_state.sizing
                    } else {
                        true
                    };
                    if use_hwnd {
                        if let Some(ref mut rt) = s.render_target {
                            if let Some(hrt) = cast_to_hwnd(rt) {
                                let size = D2D1_SIZE_U { width, height };
                                let _ = hrt.ptr.Resize(&size);
                            }
                        }
                        InvalidateRect(hwnd, null_mut(), FALSE);
                    } else {
                        let res;
                        {
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
                            let (w, h) = self.handle.borrow().pixels_to_px_xy(width, height);
                            let rect = Rect::from_origin_size(Point::ORIGIN, (w as f64, h as f64));
                            s.rebuild_render_target(&self.d2d_factory);
                            s.render(&self.d2d_factory, &self.dwrite_factory, &self.handle, rect);
                            if let Some(ref mut dcomp_state) = s.dcomp_state {
                                (*dcomp_state.swap_chain).Present(0, 0);
                                let _ = dcomp_state.dcomp_device.commit();
                            }
                            ValidateRect(hwnd, null_mut());
                        } else {
                            error!("ResizeBuffers failed: 0x{:x}", res);
                        }
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            },
            WM_COMMAND => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    s.handler.command(LOWORD(wparam as u32) as u32);
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_CHAR => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    //FIXME: this can receive lone surrogate pairs?
                    let key_code = s.stashed_key_code;

                    s.stashed_char = std::char::from_u32(wparam as u32);
                    let text = match s.stashed_char {
                        Some(c) => c,
                        None => {
                            warn!("failed to convert WM_CHAR to char: {:#X}", wparam);
                            return None;
                        }
                    };

                    let modifiers = get_mod_state();
                    let is_repeat = (lparam & 0xFFFF) > 0;
                    let event = KeyEvent::new(key_code, is_repeat, modifiers, text, text);

                    if s.handler.key_down(event) {
                        Some(0)
                    } else {
                        None
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                    None
                }
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let key_code: KeyCode = (wparam as i32).into();
                    s.stashed_key_code = key_code;

                    if key_code.is_printable() || key_code == KeyCode::Backspace {
                        //FIXME: this will fail to propogate key combinations such as alt+s
                        return None;
                    }

                    let modifiers = get_mod_state();
                    // bits 0-15 of iparam are the repeat count:
                    // https://docs.microsoft.com/en-ca/windows/desktop/inputdev/wm-keydown
                    let is_repeat = (lparam & 0xFFFF) > 0;
                    let event = KeyEvent::new(key_code, is_repeat, modifiers, "", "");

                    if s.handler.key_down(event) {
                        Some(0)
                    } else {
                        None
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                    None
                }
            }
            WM_KEYUP => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let key_code: KeyCode = (wparam as i32).into();
                    let modifiers = get_mod_state();
                    let is_repeat = false;
                    let text = s.stashed_char.take();
                    let event = KeyEvent::new(key_code, is_repeat, modifiers, text, text);
                    s.handler.key_up(event);
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            //TODO: WM_SYSCOMMAND
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                // TODO: apply mouse sensitivity based on
                // SPI_GETWHEELSCROLLLINES setting.
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let system_delta = HIWORD(wparam as u32) as i16 as f64;
                    let down_state = LOWORD(wparam as u32) as usize;
                    let mods = KeyModifiers {
                        shift: down_state & MK_SHIFT != 0,
                        alt: get_mod_state_alt(),
                        ctrl: down_state & MK_CONTROL != 0,
                        meta: get_mod_state_win(),
                    };
                    let wheel_delta = match msg {
                        WM_MOUSEWHEEL if mods.shift => Vec2::new(-system_delta, 0.),
                        WM_MOUSEWHEEL => Vec2::new(0., -system_delta),
                        WM_MOUSEHWHEEL => Vec2::new(system_delta, 0.),
                        _ => unreachable!(),
                    };

                    let mut p = POINT {
                        x: LOWORD(lparam as u32) as i16 as i32,
                        y: HIWORD(lparam as u32) as i16 as i32,
                    };
                    unsafe {
                        if ScreenToClient(hwnd, &mut p) == FALSE {
                            log::warn!(
                                "ScreenToClient failed: {}",
                                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                            );
                            return None;
                        }
                    }

                    let (px, py) = self.handle.borrow().pixels_to_px_xy(p.x, p.y);
                    let pos = Point::new(px as f64, py as f64);
                    let buttons = get_buttons(down_state);
                    let event = MouseEvent {
                        pos,
                        buttons,
                        mods,
                        count: 0,
                        focus: false,
                        button: MouseButton::None,
                        wheel_delta,
                    };
                    s.handler.wheel(&event);
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_MOUSEMOVE => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let x = LOWORD(lparam as u32) as i16 as i32;
                    let y = HIWORD(lparam as u32) as i16 as i32;

                    // When the mouse first enters the window client rect we need to register for the
                    // WM_MOUSELEAVE event. Note that WM_MOUSEMOVE is also called even when the
                    // window under the cursor changes without moving the mouse, for example when
                    // our window is first opened under the mouse cursor.
                    if !s.has_mouse_focus && is_point_in_client_rect(hwnd, x, y) {
                        let mut desc = TRACKMOUSEEVENT {
                            cbSize: mem::size_of::<TRACKMOUSEEVENT>() as DWORD,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            dwHoverTime: HOVER_DEFAULT,
                        };
                        unsafe {
                            if TrackMouseEvent(&mut desc) != FALSE {
                                s.has_mouse_focus = true;
                            } else {
                                warn!(
                                    "failed to TrackMouseEvent: {}",
                                    Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                                );
                            }
                        }
                    }

                    let (px, py) = self.handle.borrow().pixels_to_px_xy(x, y);
                    let pos = Point::new(px as f64, py as f64);
                    let mods = KeyModifiers {
                        shift: wparam & MK_SHIFT != 0,
                        alt: get_mod_state_alt(),
                        ctrl: wparam & MK_CONTROL != 0,
                        meta: get_mod_state_win(),
                    };
                    let buttons = get_buttons(wparam);
                    let event = MouseEvent {
                        pos,
                        buttons,
                        mods,
                        count: 0,
                        focus: false,
                        button: MouseButton::None,
                        wheel_delta: Vec2::ZERO,
                    };
                    s.handler.mouse_move(&event);
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_MOUSELEAVE => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    s.has_mouse_focus = false;
                    s.handler.mouse_leave();
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            // TODO: not clear where double-click processing should happen. Currently disabled
            // because CS_DBLCLKS is not set
            WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDBLCLK
            | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_MBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP
            | WM_XBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONUP => {
                let mut should_release_capture = false;
                if let Some(button) = match msg {
                    WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP => Some(MouseButton::Left),
                    WM_RBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP => Some(MouseButton::Right),
                    WM_MBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP => Some(MouseButton::Middle),
                    WM_XBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONUP => {
                        match HIWORD(wparam as u32) {
                            XBUTTON1 => Some(MouseButton::X1),
                            XBUTTON2 => Some(MouseButton::X2),
                            w => {
                                // Should never happen with current Windows
                                log::warn!("Received an unknown XBUTTON event ({})", w);
                                None
                            }
                        }
                    }
                    _ => unreachable!(),
                } {
                    if let Ok(mut s) = self.state.try_borrow_mut() {
                        let s = s.as_mut().unwrap();
                        let count = match msg {
                            WM_LBUTTONDOWN | WM_MBUTTONDOWN | WM_RBUTTONDOWN | WM_XBUTTONDOWN => 1,
                            WM_LBUTTONDBLCLK | WM_MBUTTONDBLCLK | WM_RBUTTONDBLCLK
                            | WM_XBUTTONDBLCLK => 2,
                            WM_LBUTTONUP | WM_MBUTTONUP | WM_RBUTTONUP | WM_XBUTTONUP => 0,
                            _ => unreachable!(),
                        };
                        let x = LOWORD(lparam as u32) as i16 as i32;
                        let y = HIWORD(lparam as u32) as i16 as i32;
                        let (px, py) = self.handle.borrow().pixels_to_px_xy(x, y);
                        let pos = Point::new(px as f64, py as f64);
                        let mods = KeyModifiers {
                            shift: wparam & MK_SHIFT != 0,
                            alt: get_mod_state_alt(),
                            ctrl: wparam & MK_CONTROL != 0,
                            meta: get_mod_state_win(),
                        };
                        let buttons = get_buttons(wparam);
                        let event = MouseEvent {
                            pos,
                            buttons,
                            mods,
                            count,
                            focus: false,
                            button,
                            wheel_delta: Vec2::ZERO,
                        };
                        if count > 0 {
                            s.enter_mouse_capture(hwnd, button);
                            s.handler.mouse_down(&event);
                        } else {
                            s.handler.mouse_up(&event);
                            should_release_capture = s.exit_mouse_capture(button);
                        }
                    } else {
                        self.log_dropped_msg(hwnd, msg, wparam, lparam);
                    }
                }

                // ReleaseCapture() is deferred: it needs to be called without having a mutable
                // reference to the window state, because it will generate a reentrant
                // WM_CAPTURECHANGED event.
                if should_release_capture {
                    unsafe {
                        if ReleaseCapture() == FALSE {
                            warn!(
                                "failed to release mouse capture: {}",
                                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                            );
                        }
                    }
                }

                Some(0)
            }
            DS_REQUEST_DESTROY => {
                unsafe {
                    DestroyWindow(hwnd);
                }
                Some(0)
            }
            WM_DESTROY => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    s.handler.destroy();
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_TIMER => {
                let id = wparam;
                unsafe {
                    KillTimer(hwnd, id);
                }
                let token = TimerToken::from_raw(id as u64);
                self.handle.borrow().free_timer_slot(token);
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    s.handler.timer(token);
                }
                Some(1)
            }
            WM_CAPTURECHANGED => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    s.captured_mouse_buttons.clear();
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_GETMINMAXINFO => {
                let min_max_info = unsafe { &mut *(lparam as *mut MINMAXINFO) };
                if let Ok(s) = self.state.try_borrow() {
                    let s = s.as_ref().unwrap();
                    if let Some(min_size) = s.min_size {
                        min_max_info.ptMinTrackSize.x =
                            (min_size.width * (f64::from(s.dpi) / 96.0)) as i32;
                        min_max_info.ptMinTrackSize.y =
                            (min_size.height * (f64::from(s.dpi) / 96.0)) as i32;
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            DS_RUN_IDLE => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let queue = self.handle.borrow().take_idle_queue();
                    for callback in queue {
                        match callback {
                            IdleKind::Callback(it) => it.call(s.handler.as_any()),
                            IdleKind::Token(token) => s.handler.idle(token),
                        }
                    }
                    Some(0)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app,
            handler: None,
            title: String::new(),
            menu: None,
            resizable: true,
            show_titlebar: true,
            present_strategy: Default::default(),
            size: Size::new(500.0, 400.0),
            min_size: None,
        }
    }

    /// This takes ownership, and is typically used with UiMain
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
        // TODO: Use this in `self.build`
        self.show_titlebar = show_titlebar;
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, Error> {
        unsafe {
            let class_name = super::util::CLASS_NAME.to_wide();
            let dwrite_factory = DwriteFactory::new().unwrap();
            let dw_clone = dwrite_factory.clone();
            let wndproc = MyWndProc {
                app: self.app.clone(),
                handle: Default::default(),
                d2d_factory: D2DFactory::new().unwrap(),
                dwrite_factory: dw_clone,
                state: RefCell::new(None),
                present_strategy: self.present_strategy,
            };

            let window = WindowState {
                hwnd: Cell::new(0 as HWND),
                dpi: Cell::new(0.0),
                wndproc: Box::new(wndproc),
                idle_queue: Default::default(),
                timers: Arc::new(Mutex::new(TimerSlots::new(1))),
            };
            let win = Rc::new(window);
            let handle = WindowHandle {
                dwrite_factory,
                state: Rc::downgrade(&win),
            };

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

            let state = WndState {
                handler: self.handler.unwrap(),
                render_target: None,
                dcomp_state: None,
                dpi,
                min_size: self.min_size,
                stashed_key_code: KeyCode::Unknown(0),
                stashed_char: None,
                captured_mouse_buttons: MouseButtons::new(),
                has_mouse_focus: false,
            };
            win.wndproc.connect(&handle, state);

            let width = (self.size.width * (f64::from(dpi) / 96.0)) as i32;
            let height = (self.size.height * (f64::from(dpi) / 96.0)) as i32;

            let (hmenu, accels) = match self.menu {
                Some(menu) => {
                    let accels = menu.accels();
                    (menu.into_hmenu(), accels)
                }
                None => (0 as HMENU, None),
            };

            let mut dwStyle = WS_OVERLAPPEDWINDOW;
            if !self.resizable {
                dwStyle &= !(WS_THICKFRAME | WS_MAXIMIZEBOX);
            }

            let mut dwExStyle = 0;
            if self.present_strategy == PresentStrategy::Flip {
                dwExStyle |= WS_EX_NOREDIRECTIONBITMAP;
            }
            let hwnd = create_window(
                dwExStyle,
                class_name.as_ptr(),
                self.title.to_wide().as_ptr(),
                dwStyle,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                0 as HWND,
                hmenu,
                0 as HINSTANCE,
                win,
            );
            if hwnd.is_null() {
                return Err(Error::NullHwnd);
            }
            self.app.add_window(hwnd);

            if let Some(accels) = accels {
                register_accel(hwnd, &accels);
            }
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
        let mut desc = mem::MaybeUninit::uninit();
        let hr = (*adapter).GetDesc(desc.as_mut_ptr());
        if !SUCCEEDED(hr) {
            error!("Failed to get adapter description: {:?}", Error::Hr(hr));
            break;
        }
        let mut desc: DXGI_ADAPTER_DESC = desc.assume_init();
        let vram = desc.DedicatedVideoMemory;
        if i == 0 || vram > best_vram {
            best_vram = vram;
            best_adapter = adapter;
        }
        debug!(
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
        debug!("dxgi factory pointer = {:?}", factory);
        let adapter = choose_adapter(factory);
        debug!("adapter = {:?}", adapter);

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
        debug!("swap chain res = 0x{:x}, pointer = {:?}", res, swap_chain);

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

#[cfg(target_arch = "x86_64")]
type WindowLongPtr = winapi::shared::basetsd::LONG_PTR;
#[cfg(target_arch = "x86")]
type WindowLongPtr = LONG;

pub(crate) unsafe extern "system" fn win_proc_dispatch(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let create_struct = &*(lparam as *const CREATESTRUCTW);
        let wndproc_ptr = create_struct.lpCreateParams;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, wndproc_ptr as WindowLongPtr);
    }
    let window_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
    let result = {
        if window_ptr.is_null() {
            None
        } else {
            (*window_ptr).wndproc.window_proc(hwnd, msg, wparam, lparam)
        }
    };

    if msg == WM_NCDESTROY && !window_ptr.is_null() {
        (*window_ptr).wndproc.cleanup(hwnd);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        mem::drop(Rc::from_raw(window_ptr));
    }

    match result {
        Some(lresult) => lresult,
        None => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Create a window (same parameters as CreateWindowExW) with associated WndProc.
#[allow(clippy::too_many_arguments)]
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
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                ShowWindow(hwnd, SW_SHOWNORMAL);
                UpdateWindow(hwnd);
            }
        }
    }

    pub fn close(&self) {
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                PostMessageW(hwnd, DS_REQUEST_DESTROY, 0, 0);
            }
        }
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        //FIXME: implementation goes here
        log::warn!("bring_to_front_and_focus not yet implemented on windows");
    }

    pub fn invalidate(&self) {
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                InvalidateRect(hwnd, null(), FALSE);
            }
        }
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        let rect = self.px_to_rect(rect);
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                if InvalidateRect(hwnd, &rect, FALSE) == FALSE {
                    log::warn!(
                        "InvalidateRect failed: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                }
            }
        }
    }

    /// Set the title for this menu.
    pub fn set_title(&self, title: &str) {
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                if SetWindowTextW(hwnd, title.to_wide().as_ptr()) == FALSE {
                    warn!("failed to set window title '{}'", title);
                }
            }
        }
    }

    // TODO: Implement this
    pub fn show_titlebar(&self, _show_titlebar: bool) {}

    pub fn resizable(&self, resizable: bool) {
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                if style == 0 {
                    warn!(
                        "failed to get window style: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                    return;
                }

                if resizable {
                    style |= (WS_THICKFRAME | WS_MAXIMIZEBOX) as WindowLongPtr;
                } else {
                    style &= !(WS_THICKFRAME | WS_MAXIMIZEBOX) as WindowLongPtr;
                }

                if SetWindowLongPtrW(hwnd, GWL_STYLE, style) == 0 {
                    warn!(
                        "failed to set the window style: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                }
            }
        }
    }

    pub fn set_menu(&self, menu: Menu) {
        let accels = menu.accels();
        let hmenu = menu.into_hmenu();
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                let old_menu = GetMenu(hwnd);
                if SetMenu(hwnd, hmenu) == FALSE {
                    warn!("failed to set window menu");
                } else {
                    DestroyMenu(old_menu);
                }
                if let Some(accels) = accels {
                    register_accel(hwnd, &accels);
                }
            }
        }
    }

    pub fn show_context_menu(&self, menu: Menu, pos: Point) {
        let hmenu = menu.into_hmenu();
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            let (x, y) = self.px_to_pixels_xy(pos.x as f32, pos.y as f32);
            unsafe {
                let mut point = POINT { x, y };
                ClientToScreen(hwnd, &mut point);
                if TrackPopupMenu(hmenu, TPM_LEFTALIGN, point.x, point.y, 0, hwnd, null()) == FALSE
                {
                    warn!("failed to track popup menu");
                }
            }
        }
    }

    pub fn text(&self) -> Text {
        Text::new(&self.dwrite_factory)
    }

    /// Request a timer event.
    ///
    /// The return value is an identifier.
    pub fn request_timer(&self, deadline: std::time::Instant) -> TimerToken {
        let (id, elapse) = self.get_timer_slot(deadline);
        let id = self
            .get_hwnd()
            // we reuse timer ids; if this is greater than u32::max we have a problem.
            .map(|hwnd| unsafe { SetTimer(hwnd, id.into_raw() as usize, elapse, None) as u64 })
            .unwrap_or(0);
        TimerToken::from_raw(id)
    }

    /// Set the cursor icon.
    pub fn set_cursor(&mut self, cursor: &Cursor) {
        unsafe {
            let cursor = LoadCursorW(0 as HINSTANCE, cursor.get_lpcwstr());
            SetCursor(cursor);
        }
    }

    //FIXME: these two methods will be reworked to avoid reentrancy problems.
    // Currently, calling it may result in important messages being dropped.
    /// Prompt the user to chose a file to open.
    ///
    /// Blocks while the user picks the file.
    pub fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        let hwnd = self.get_hwnd()?;
        unsafe {
            get_file_dialog_path(hwnd, FileDialogType::Open, options)
                .ok()
                .map(|s| FileInfo { path: s.into() })
        }
    }

    /// Prompt the user to chose a file to open.
    ///
    /// Blocks while the user picks the file.
    pub fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        let hwnd = self.get_hwnd()?;
        unsafe {
            get_file_dialog_path(hwnd, FileDialogType::Save, options)
                .ok()
                .map(|os_str| FileInfo {
                    path: os_str.into(),
                })
        }
    }

    /// Get the raw HWND handle, for uses that are not wrapped in
    /// druid_win_shell.
    pub fn get_hwnd(&self) -> Option<HWND> {
        self.state.upgrade().map(|w| w.hwnd.get())
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        self.state.upgrade().map(|w| IdleHandle {
            hwnd: w.hwnd.get(),
            queue: w.idle_queue.clone(),
        })
    }

    fn take_idle_queue(&self) -> Vec<IdleKind> {
        if let Some(w) = self.state.upgrade() {
            mem::replace(&mut w.idle_queue.lock().unwrap(), Vec::new())
        } else {
            Vec::new()
        }
    }

    /// Get the dpi of the window.
    pub fn get_dpi(&self) -> f32 {
        if let Some(w) = self.state.upgrade() {
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

    /// Convert a rectangle from physical pixels to px units.
    pub fn rect_to_px(&self, rect: RECT) -> Rect {
        let (x0, y0) = self.pixels_to_px_xy(rect.left, rect.top);
        let (x1, y1) = self.pixels_to_px_xy(rect.right, rect.bottom);
        Rect::new(x0 as f64, y0 as f64, x1 as f64, y1 as f64)
    }

    pub fn px_to_rect(&self, rect: Rect) -> RECT {
        let scale = self.get_dpi() as f64 / 96.0;
        RECT {
            left: (rect.x0 * scale).floor() as i32,
            top: (rect.y0 * scale).floor() as i32,
            right: (rect.x1 * scale).ceil() as i32,
            bottom: (rect.y1 * scale).ceil() as i32,
        }
    }

    /// Allocate a timer slot.
    ///
    /// Returns an id and an elapsed time in ms
    fn get_timer_slot(&self, deadline: std::time::Instant) -> (TimerToken, u32) {
        if let Some(w) = self.state.upgrade() {
            let mut timers = w.timers.lock().unwrap();
            let id = timers.alloc();
            let elapsed = timers.compute_elapsed(deadline);
            (id, elapsed)
        } else {
            (TimerToken::INVALID, 0)
        }
    }

    fn free_timer_slot(&self, token: TimerToken) {
        if let Some(w) = self.state.upgrade() {
            w.timers.lock().unwrap().free(token)
        }
    }
}

// There is a tiny risk of things going wrong when hwnd is sent across threads.
unsafe impl Send for IdleHandle {}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the window's wndproc,
    /// which means it won't be scheduled if the window is closed.
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            unsafe {
                PostMessageW(self.hwnd, DS_RUN_IDLE, 0, 0);
            }
        }
        queue.push(IdleKind::Callback(Box::new(callback)));
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            unsafe {
                PostMessageW(self.hwnd, DS_RUN_IDLE, 0, 0);
            }
        }
        queue.push(IdleKind::Token(token));
    }

    fn invalidate(&self) {
        unsafe {
            InvalidateRect(self.hwnd, null(), FALSE);
        }
    }
}

/// Casts render target to hwnd variant.
unsafe fn cast_to_hwnd(dc: &DeviceContext) -> Option<HwndRenderTarget> {
    dc.get_comptr()
        .cast()
        .ok()
        .map(|com_ptr| HwndRenderTarget::from_ptr(com_ptr))
}

impl Default for WindowHandle {
    fn default() -> Self {
        WindowHandle {
            state: Default::default(),
            dwrite_factory: DwriteFactory::new().unwrap(),
        }
    }
}
