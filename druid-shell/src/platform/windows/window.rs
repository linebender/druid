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
use std::ops::Deref;
use std::ptr::{null, null_mut};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use log::{debug, error, warn};
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
use winapi::um::winnt::*;
use winapi::um::winuser::*;
use winapi::Interface;

use direct2d;
use direct2d::math::SizeU;
use direct2d::render_target::{GenericRenderTarget, HwndRenderTarget, RenderTarget};

use crate::kurbo::{Point, Size, Vec2};
use crate::piet::{Piet, RenderContext};

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
use crate::mouse::{Cursor, MouseButton, MouseEvent};
use crate::window::{Text, TimerToken, WinCtx, WinHandler};

extern "system" {
    pub fn DwmFlush();
}

/// Builder abstraction for creating new windows.
pub struct WindowBuilder {
    handler: Option<Box<dyn WinHandler>>,
    dwStyle: DWORD,
    title: String,
    menu: Option<Menu>,
    present_strategy: PresentStrategy,
    size: Size,
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

#[derive(Default)]
pub struct WindowHandle {
    // Note: this clone of the dwrite factory might move into WinCtxImpl.
    dwrite_factory: Option<directwrite::Factory>,
    state: Weak<WindowState>,
}

/// A handle that can get used to schedule an idle handler. Note that
/// this handle is thread safe. If the handle is used after the hwnd
/// has been destroyed, probably not much will go wrong (the XI_RUN_IDLE
/// message may be sent to a stray window).
#[derive(Clone)]
pub struct IdleHandle {
    pub(crate) hwnd: HWND,
    queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,
}

/// This is the low level window state. All mutable contents are protected
/// by interior mutability, so we can handle reentrant calls.
struct WindowState {
    hwnd: Cell<HWND>,
    dpi: Cell<f32>,
    wndproc: Box<dyn WndProc>,
    idle_queue: Arc<Mutex<Vec<Box<dyn IdleCallback>>>>,

    // This field doesn't really need to be shared; it could be plumbed
    // as a mutable reference down through WinCtx, but that would require
    // some refactoring.
    timers: Arc<Mutex<TimerSlots>>,
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
    handle: RefCell<WindowHandle>,
    d2d_factory: direct2d::Factory,
    dwrite_factory: directwrite::Factory,
    state: RefCell<Option<WndState>>,
}

/// The mutable state of the window.
struct WndState {
    handler: Box<dyn WinHandler>,
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

/// A structure that owns resources for the `WinCtx` (so it lasts long enough).
struct WinCtxOwner<'a> {
    handle: std::cell::Ref<'a, WindowHandle>,
    dwrite: &'a directwrite::Factory,
}

/// The Windows implementation of the context provided to WinHandler calls.
struct WinCtxImpl<'a> {
    handle: &'a WindowHandle,
    text: Text<'a>,
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

impl WndState {
    fn rebuild_render_target(&mut self, d2d: &direct2d::Factory) {
        unsafe {
            let swap_chain = self.dcomp_state.as_ref().unwrap().swap_chain;
            let rt = paint::create_render_target_dxgi(d2d, swap_chain, self.dpi)
                .map(|rt| rt.as_generic());
            self.render_target = rt.ok();
        }
    }

    // Renders but does not present.
    fn render(
        &mut self,
        d2d: &direct2d::Factory,
        dw: &directwrite::Factory,
        handle: &RefCell<WindowHandle>,
        c: &mut WinCtxOwner,
    ) {
        let rt = self.render_target.as_mut().unwrap();
        rt.begin_draw();
        let anim;
        {
            let mut piet_ctx = Piet::new(d2d, dw, rt);
            anim = self.handler.paint(&mut piet_ctx, &mut c.ctx());
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
            handle.add_idle(move |_| handle2.invalidate());
        }
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

impl<'a> WinCtxOwner<'a> {
    fn new(
        handle: std::cell::Ref<'a, WindowHandle>,
        dwrite: &'a directwrite::Factory,
    ) -> WinCtxOwner<'a> {
        WinCtxOwner { handle, dwrite }
    }

    fn ctx<'b>(&'b mut self) -> WinCtxImpl<'b>
    where
        'a: 'b,
    {
        let text = Text::new(&self.dwrite);
        WinCtxImpl {
            handle: self.handle.deref(),
            text,
        }
    }
}

impl WndProc for MyWndProc {
    fn connect(&self, handle: &WindowHandle, mut state: WndState) {
        *self.handle.borrow_mut() = handle.clone();
        state.handler.connect(&handle.clone().into());
        *self.state.borrow_mut() = Some(state);
        if let Ok(mut s) = self.state.try_borrow_mut() {
            let mut ctx = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
            s.as_mut().unwrap().handler.connected(&mut ctx.ctx());
        }
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
            WM_ERASEBKGND => Some(0),
            WM_SETFOCUS => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.got_focus(&mut c.ctx());
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_PAINT => unsafe {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    if s.render_target.is_none() {
                        let rt = paint::create_render_target(&self.d2d_factory, hwnd)
                            .map(|rt| rt.as_generic());
                        s.render_target = rt.ok();
                    }
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.rebuild_resources(&mut c.ctx());
                    s.render(
                        &self.d2d_factory,
                        &self.dwrite_factory,
                        &self.handle,
                        &mut c,
                    );
                    if let Some(ref mut ds) = s.dcomp_state {
                        if !ds.sizing {
                            (*ds.swap_chain).Present(1, 0);
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
                        let rt = paint::create_render_target(&self.d2d_factory, hwnd)
                            .map(|rt| rt.as_generic());
                        s.render_target = rt.ok();
                        {
                            let mut c =
                                WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                            s.handler.rebuild_resources(&mut c.ctx());
                            s.render(
                                &self.d2d_factory,
                                &self.dwrite_factory,
                                &self.handle,
                                &mut c,
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
                        if GetClientRect(hwnd, &mut rect) == 0 {
                            warn!("GetClientRect failed.");
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
                            let mut c =
                                WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                            s.handler.rebuild_resources(&mut c.ctx());
                            s.rebuild_render_target(&self.d2d_factory);
                            let mut c =
                                WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                            s.render(
                                &self.d2d_factory,
                                &self.dwrite_factory,
                                &self.handle,
                                &mut c,
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
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.size(width, height, &mut c.ctx());
                    let use_hwnd = if let Some(ref dcomp_state) = s.dcomp_state {
                        dcomp_state.sizing
                    } else {
                        true
                    };
                    if use_hwnd {
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
                            s.rebuild_render_target(&self.d2d_factory);
                            let mut c =
                                WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                            s.render(
                                &self.d2d_factory,
                                &self.dwrite_factory,
                                &self.handle,
                                &mut c,
                            );
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
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler
                        .command(LOWORD(wparam as u32) as u32, &mut c.ctx());
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

                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    if s.handler.key_down(event, &mut c.ctx()) {
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

                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    if s.handler.key_down(event, &mut c.ctx()) {
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
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    let event = KeyEvent::new(key_code, is_repeat, modifiers, text, text);
                    s.handler.key_up(event, &mut c.ctx());
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            //TODO: WM_SYSCOMMAND
            WM_MOUSEWHEEL => {
                // TODO: apply mouse sensitivity based on
                // SPI_GETWHEELSCROLLLINES setting.
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let delta_y = HIWORD(wparam as u32) as i16 as f64;
                    let delta = Vec2::new(0.0, -delta_y);
                    let mods = get_mod_state();
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.wheel(delta, mods, &mut c.ctx());
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_MOUSEHWHEEL => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let delta_x = HIWORD(wparam as u32) as i16 as f64;
                    let delta = Vec2::new(delta_x, 0.0);
                    let mods = get_mod_state();
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.wheel(delta, mods, &mut c.ctx());
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
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.mouse_move(&event, &mut c.ctx());
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            // TODO: not clear where double-click processing should happen. Currently disabled
            // because CS_DBLCLKS is not set
            WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MBUTTONDBLCLK
            | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_RBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP
            | WM_XBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONUP => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let button = match msg {
                        WM_LBUTTONDBLCLK | WM_LBUTTONDOWN | WM_LBUTTONUP => MouseButton::Left,
                        WM_MBUTTONDBLCLK | WM_MBUTTONDOWN | WM_MBUTTONUP => MouseButton::Middle,
                        WM_RBUTTONDBLCLK | WM_RBUTTONDOWN | WM_RBUTTONUP => MouseButton::Right,
                        WM_XBUTTONDBLCLK | WM_XBUTTONDOWN | WM_XBUTTONUP => {
                            match HIWORD(wparam as u32) {
                                1 => MouseButton::X1,
                                2 => MouseButton::X2,
                                _ => {
                                    warn!("unexpected X button event");
                                    return None;
                                }
                            }
                        }
                        _ => unreachable!(),
                    };
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
                    let mods = get_mod_state();
                    let event = MouseEvent {
                        pos,
                        mods,
                        button,
                        count,
                    };
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    if count > 0 {
                        s.handler.mouse_down(&event, &mut c.ctx());
                    } else {
                        s.handler.mouse_up(&event, &mut c.ctx());
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            }
            WM_DESTROY => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.destroy(&mut c.ctx());
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                None
            }
            WM_TIMER => {
                let id = wparam;
                unsafe {
                    KillTimer(hwnd, id);
                }
                let token = TimerToken::new(id);
                self.handle.borrow().free_timer_slot(token);
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let mut c = WinCtxOwner::new(self.handle.borrow(), &self.dwrite_factory);
                    s.handler.timer(token, &mut c.ctx());
                }
                Some(1)
            }
            XI_RUN_IDLE => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let queue = self.handle.borrow().take_idle_queue();
                    let handler_as_any = s.handler.as_any();
                    for callback in queue {
                        callback.call(handler_as_any);
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

// Note: there's a clone method in 0.3.0-alpha4. We work around
// the lack in 0.1.2 by calling the low-level unsafe operations.
fn clone_dwrite(dwrite: &directwrite::Factory) -> directwrite::Factory {
    unsafe {
        (*dwrite.get_raw()).AddRef();
        directwrite::Factory::from_raw(dwrite.get_raw())
    }
}

impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            handler: None,
            dwStyle: WS_OVERLAPPEDWINDOW,
            title: String::new(),
            menu: None,
            present_strategy: Default::default(),
            size: Size::new(500.0, 400.0),
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

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
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

            let class_name = super::util::CLASS_NAME.to_wide();
            let dwrite_factory = directwrite::Factory::new().unwrap();
            let dw_clone = clone_dwrite(&dwrite_factory);
            let wndproc = MyWndProc {
                handle: Default::default(),
                d2d_factory: direct2d::Factory::new().unwrap(),
                dwrite_factory: dw_clone,
                state: RefCell::new(None),
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
                dwrite_factory: Some(dwrite_factory),
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
            let width = (self.size.width * (f64::from(dpi) / 96.0)) as i32;
            let height = (self.size.height * (f64::from(dpi) / 96.0)) as i32;

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
                return Err(Error::NullHwnd);
            }

            let dcomp_state = create_dcomp_state(self.present_strategy, hwnd).unwrap_or_else(|e| {
                warn!("Creating swapchain failed, falling back to hwnd: {:?}", e);
                None
            });

            win.hwnd.set(hwnd);
            let state = WndState {
                handler: self.handler.unwrap(),
                render_target: None,
                dcomp_state,
                dpi,
                stashed_key_code: KeyCode::Unknown(0),
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

pub(crate) unsafe extern "system" fn win_proc_dispatch(
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

    if msg == WM_NCDESTROY && !window_ptr.is_null() {
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

// TODO: when upgrading to directwrite 0.3, just derive Clone instead.
impl Clone for WindowHandle {
    fn clone(&self) -> WindowHandle {
        let dw_clone = self.dwrite_factory.as_ref().map(|dw| clone_dwrite(dw));
        WindowHandle {
            dwrite_factory: dw_clone,
            state: self.state.clone(),
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
                DestroyWindow(hwnd);
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

    pub fn set_menu(&self, menu: Menu) {
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

    fn take_idle_queue(&self) -> Vec<Box<dyn IdleCallback>> {
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

// Note: this has mostly methods moved from `WindowHandle`, so mostly forwards
// to those. As a cleanup, some may be implemented more directly.
impl<'a> WinCtx<'a> for WinCtxImpl<'a> {
    fn invalidate(&mut self) {
        self.handle.invalidate();
    }

    /// Get a reference to the text factory.
    fn text_factory(&mut self) -> &mut Text<'a> {
        &mut self.text
    }

    /// Set the cursor icon.
    fn set_cursor(&mut self, cursor: &Cursor) {
        unsafe {
            let cursor = LoadCursorW(0 as HINSTANCE, cursor.get_lpcwstr());
            SetCursor(cursor);
        }
    }

    /// Request a timer event.
    ///
    /// The return value is an identifier.
    fn request_timer(&mut self, deadline: std::time::Instant) -> TimerToken {
        let id = self
            .handle
            .get_hwnd()
            .map(|hwnd| {
                let (id, elapse) = self.handle.get_timer_slot(deadline);
                unsafe {
                    let id = SetTimer(hwnd, id.get_raw(), elapse, None);
                    id as usize
                }
            })
            .unwrap_or(0);
        TimerToken::new(id)
    }

    //FIXME: these two methods will be reworked to avoid reentrancy problems.
    // Currently, calling it may result in important messages being dropped.
    /// Prompt the user to chose a file to open.
    ///
    /// Blocks while the user picks the file.
    fn open_file_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        let hwnd = self.handle.get_hwnd()?;
        unsafe {
            get_file_dialog_path(hwnd, FileDialogType::Open, options)
                .ok()
                .map(|os_str| FileInfo {
                    path: os_str.into(),
                })
        }
    }

    /// Prompt the user to chose a file to open.
    ///
    /// Blocks while the user picks the file.
    fn save_as_sync(&mut self, options: FileDialogOptions) -> Option<FileInfo> {
        let hwnd = self.handle.get_hwnd()?;
        unsafe {
            get_file_dialog_path(hwnd, FileDialogType::Save, options)
                .ok()
                .map(|os_str| FileInfo {
                    path: os_str.into(),
                })
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
