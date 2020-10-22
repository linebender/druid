// Copyright 2018 The Druid Authors.
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
use std::time::{Duration, Instant};

use log::{debug, error, warn};
use scopeguard::defer;
use winapi::ctypes::{c_int, c_void};
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::shellscalingapi::MDT_EFFECTIVE_DPI;
use winapi::um::unknwnbase::*;
use winapi::um::wingdi::*;
use winapi::um::winnt::*;
use winapi::um::winuser::*;

use piet_common::d2d::{D2DFactory, DeviceContext};
use piet_common::dwrite::DwriteFactory;

use crate::kurbo::{Point, Rect, Size, Vec2};
use crate::piet::{Piet, PietText, RenderContext};

use super::accels::register_accel;
use super::application::Application;
use super::dcomp::D3D11Device;
use super::dialog::get_file_dialog_path;
use super::error::Error;
use super::keyboard::KeyboardState;
use super::menu::Menu;
use super::paint;
use super::timers::TimerSlots;
use super::util::{self, as_result, FromWide, ToWide, OPTIONAL_FUNCTIONS};

use crate::common_util::IdleCallback;
use crate::dialog::{FileDialogOptions, FileDialogType, FileInfo};
use crate::error::Error as ShellError;
use crate::keyboard::{KbKey, KeyState};
use crate::mouse::{Cursor, CursorDesc, MouseButton, MouseButtons, MouseEvent};
use crate::region::Region;
use crate::scale::{Scalable, Scale, ScaledArea};
use crate::window;
use crate::window::{FileDialogToken, IdleToken, TimerToken, WinHandler, WindowLevel};

/// The platform target DPI.
///
/// Windows considers 96 the default value which represents a 1.0 scale factor.
pub(crate) const SCALE_TARGET_DPI: f64 = 96.0;

/// Builder abstraction for creating new windows.
pub(crate) struct WindowBuilder {
    app: Application,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    present_strategy: PresentStrategy,
    resizable: bool,
    show_titlebar: bool,
    size: Option<Size>,
    min_size: Option<Size>,
    position: Point,
    state: window::WindowState,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// It's very tricky to get smooth dynamics (especially resizing) and
/// good performance on Windows. This setting lets clients experiment
/// with different strategies.
pub enum PresentStrategy {
    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_SEQUENTIAL. It
    /// is compatible with GDI (such as menus), but is not the best in
    /// performance.
    ///
    /// In earlier testing, it exhibited diagonal banding artifacts (most
    /// likely because of bugs in Nvidia Optimus configurations) and did
    /// not do incremental present, but in more recent testing, at least
    /// incremental present seems to work fine.
    ///
    /// Also note, this swap effect is not compatible with DX12.
    Sequential,

    /// Corresponds to the swap effect DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL.
    /// In testing, it seems to perform well, but isn't compatible with
    /// GDI. Resize can probably be made to work reasonably smoothly with
    /// additional synchronization work, but has some artifacts.
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

#[derive(Clone)]
enum DeferredOp {
    SetPosition(Point),
    SetSize(Size),
    DecorationChanged(),
    // Needs a better name
    SetWindowSizeState(window::WindowState),
}

struct DeferredQueue {
    queue: Vec<DeferredOp>,
}

impl DeferredQueue {
    pub fn new() -> Self {
        DeferredQueue { queue: Vec::new() }
    }

    /// Adds a DeferredOp message to the queue.
    /// The message will be run at a later time.
    pub fn add(state: Weak<WindowState>, message: DeferredOp) {
        if let Some(w) = state.upgrade() {
            if let Ok(mut q) = w.deferred_queue.try_borrow_mut() {
                q.queue.push(message)
            } else {
                warn!("failed to borrow deferred queue");
            }
        }
    }

    /// Empties the current message queue, returning a queue with the messages.
    pub fn empty(&mut self) -> Vec<DeferredOp> {
        let ret = self.queue.clone();
        self.queue = Vec::new();
        ret
    }

    // Here we handle messages generated by WindowHandle
    // that needs to run after borrow is released
    fn handle_deferred(proc: &MyWndProc, op: DeferredOp) {
        if let Some(hwnd) = proc.handle.borrow().get_hwnd() {
            match op {
                DeferredOp::SetSize(size) => unsafe {
                    if SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        0,
                        0,
                        (size.width * proc.scale().x()) as i32,
                        (size.height * proc.scale().y()) as i32,
                        SWP_NOMOVE | SWP_NOZORDER,
                    ) == 0
                    {
                        warn!(
                            "failed to resize window: {}",
                            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                        );
                    };
                },
                DeferredOp::SetPosition(position) => unsafe {
                    if SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        position.x as i32,
                        position.y as i32,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER,
                    ) == 0
                    {
                        warn!(
                            "failed to move window: {}",
                            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                        );
                    };
                },
                DeferredOp::DecorationChanged() => unsafe {
                    let resizable = proc.resizable();
                    let titlebar = proc.has_titlebar();

                    let mut style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
                    if style == 0 {
                        warn!(
                            "failed to get window style: {}",
                            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                        );
                        return;
                    }

                    if !resizable {
                        style &= !(WS_THICKFRAME | WS_MAXIMIZEBOX);
                    } else {
                        style |= WS_THICKFRAME | WS_MAXIMIZEBOX;
                    }
                    if !titlebar {
                        style &= !(WS_MINIMIZEBOX | WS_SYSMENU | WS_OVERLAPPED);
                    } else {
                        style |= WS_MINIMIZEBOX | WS_SYSMENU | WS_OVERLAPPED;
                    }
                    if SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize) == 0 {
                        warn!(
                            "failed to set the window style: {}",
                            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                        );
                    }
                    if SetWindowPos(
                        hwnd,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOSIZE,
                    ) == 0
                    {
                        warn!(
                            "failed to update window style: {}",
                            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                        );
                    };
                },
                DeferredOp::SetWindowSizeState(val) => unsafe {
                    let s = match val {
                        window::WindowState::MAXIMIZED => SW_MAXIMIZE,
                        window::WindowState::MINIMIZED => SW_MINIMIZE,
                        window::WindowState::RESTORED => SW_RESTORE,
                    };
                    ShowWindow(hwnd, s);
                },
            }
        } else {
            warn!("Could not get HWND");
        }
    }
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
    scale: Cell<Scale>,
    area: Cell<ScaledArea>,
    invalid: RefCell<Region>,
    has_menu: Cell<bool>,
    wndproc: Box<dyn WndProc>,
    idle_queue: Arc<Mutex<Vec<IdleKind>>>,
    timers: Arc<Mutex<TimerSlots>>,
    deferred_queue: RefCell<DeferredQueue>,
    has_titlebar: Cell<bool>,
    // For resizable borders, window can still be resized with code.
    is_resizable: Cell<bool>,
    handle_titlebar: Cell<bool>,
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
    dxgi_state: Option<DxgiState>,
    min_size: Option<Size>,
    keyboard_state: KeyboardState,
    // Stores a set of all mouse buttons that are currently holding mouse
    // capture. When the first mouse button is down on our window we enter
    // capture, and we hold it until the last mouse button is up.
    captured_mouse_buttons: MouseButtons,
    // Is this window the topmost window under the mouse cursor
    has_mouse_focus: bool,
    //TODO: track surrogate orphan
    last_click_time: Instant,
    last_click_pos: (i32, i32),
    click_count: u8,
}

/// State for DXGI swapchains.
struct DxgiState {
    swap_chain: *mut IDXGISwapChain1,
}

#[derive(Clone)]
pub struct CustomCursor(Arc<HCursor>);

struct HCursor(HCURSOR);

impl Drop for HCursor {
    fn drop(&mut self) {
        unsafe {
            DestroyIcon(self.0);
        }
    }
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

/// A message which must be delivered deferred due to reentrancy.
///
/// Due to the architecture of Windows, it sometimes delivers messages
/// reentrantly. This is problematic when the mutable state for the window
/// is borrowed, as it's not possible to safely dispatch the message. The two
/// choices are to drop the message, or put it in a queue for deferred
/// processing.
///
/// This mechanism should be used very sparingly, as delivery of these
/// deferred messages can not be guaranteed in a timely and reliable
/// fashion. In particular, we do not run our own runloop when handling
/// live resize, when a modal dialog is open, or when the application is a
/// a guest (such as a VST plugin).
pub(crate) const DS_HANDLE_DEFERRED: UINT = WM_USER + 2;

impl Default for PresentStrategy {
    fn default() -> PresentStrategy {
        PresentStrategy::Sequential
    }
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
    fn rebuild_render_target(&mut self, d2d: &D2DFactory, scale: Scale) {
        unsafe {
            let swap_chain = self.dxgi_state.as_ref().unwrap().swap_chain;
            let rt = paint::create_render_target_dxgi(d2d, swap_chain, scale)
                .map(|rt| rt.as_device_context().expect("TODO remove this expect"));
            self.render_target = rt.ok();
        }
    }

    // Renders but does not present.
    fn render(&mut self, d2d: &D2DFactory, dw: &DwriteFactory, invalid: &Region) {
        let rt = self.render_target.as_mut().unwrap();
        rt.begin_draw();
        {
            let mut piet_ctx = Piet::new(d2d, dw.clone(), rt);
            // The documentation on DXGI_PRESENT_PARAMETERS says we "must not update any
            // pixel outside of the dirty rectangles."
            piet_ctx.clip(invalid.to_bez_path());
            self.handler.paint(&mut piet_ctx, invalid);
            if let Err(e) = piet_ctx.finish() {
                error!("piet error on render: {:?}", e);
            }
        }
        // Maybe should deal with lost device here...
        let res = rt.end_draw();
        if let Err(e) = res {
            error!("EndDraw error: {:?}", e);
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

    fn with_window_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(Rc<WindowState>) -> R,
    {
        f(self
            .handle
            // Right now there aren't any mutable borrows to this.
            // TODO: Attempt to guarantee this by making mutable handle borrows useless.
            .borrow()
            .state
            .upgrade()
            .unwrap()) // WindowState drops after WM_NCDESTROY, so it's always here.
    }

    fn scale(&self) -> Scale {
        self.with_window_state(|state| state.scale.get())
    }

    fn set_scale(&self, scale: Scale) {
        self.with_window_state(move |state| state.scale.set(scale))
    }

    /// Takes the invalid region and returns it, replacing it with the empty region.
    fn take_invalid(&self) -> Region {
        self.with_window_state(|state| {
            std::mem::replace(&mut *state.invalid.borrow_mut(), Region::EMPTY)
        })
    }

    fn invalidate_rect(&self, rect: Rect) {
        self.with_window_state(|state| state.invalid.borrow_mut().add_rect(rect));
    }

    fn set_area(&self, area: ScaledArea) {
        self.with_window_state(move |state| state.area.set(area))
    }

    fn has_menu(&self) -> bool {
        self.with_window_state(|state| state.has_menu.get())
    }

    fn has_titlebar(&self) -> bool {
        self.with_window_state(|state| state.has_titlebar.get())
    }

    fn resizable(&self) -> bool {
        self.with_window_state(|state| state.is_resizable.get())
    }

    fn handle_deferred_queue(&self) {
        let q = self.with_window_state(move |state| state.deferred_queue.borrow_mut().empty());
        for op in q {
            DeferredQueue::handle_deferred(self, op);
        }
    }
}

impl WndProc for MyWndProc {
    fn connect(&self, handle: &WindowHandle, state: WndState) {
        *self.handle.borrow_mut() = handle.clone();
        *self.state.borrow_mut() = Some(state);
        self.state
            .borrow_mut()
            .as_mut()
            .unwrap()
            .handler
            .scale(self.scale());
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
                // Only supported on Windows 10, Could remove this as the 8.1 version below also works on 10..
                let scale_factor = if let Some(func) = OPTIONAL_FUNCTIONS.GetDpiForWindow {
                    unsafe { func(hwnd) as f64 / SCALE_TARGET_DPI }
                }
                // Windows 8.1 Support
                else if let Some(func) = OPTIONAL_FUNCTIONS.GetDpiForMonitor {
                    unsafe {
                        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                        let mut dpiX = 0;
                        let mut dpiY = 0;
                        func(monitor, MDT_EFFECTIVE_DPI, &mut dpiX, &mut dpiY);
                        dpiX as f64 / SCALE_TARGET_DPI
                    }
                } else {
                    1.0
                };

                let scale = Scale::new(scale_factor, scale_factor);
                self.set_scale(scale);

                if let Some(state) = self.handle.borrow().state.upgrade() {
                    state.hwnd.set(hwnd);
                }
                if let Some(state) = self.state.borrow_mut().as_mut() {
                    let dxgi_state = unsafe {
                        create_dxgi_state(self.present_strategy, hwnd).unwrap_or_else(|e| {
                            error!("Creating swapchain failed: {:?}", e);
                            None
                        })
                    };
                    state.dxgi_state = dxgi_state;

                    let handle = self.handle.borrow().to_owned();
                    state.handler.connect(&handle.into());
                }
                Some(0)
            }
            WM_ACTIVATE => {
                if LOWORD(wparam as u32) as u32 != 0 {
                    unsafe {
                        if SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_SHOWWINDOW
                                | SWP_NOMOVE
                                | SWP_NOZORDER
                                | SWP_FRAMECHANGED
                                | SWP_NOSIZE,
                        ) == 0
                        {
                            warn!(
                                "failed to update window style: {}",
                                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                            );
                        };
                    }
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
                    let s = s.as_mut().unwrap();
                    // We call prepare_paint before GetUpdateRect, so that anything invalidated during
                    // prepare_paint will be reflected in GetUpdateRect.
                    s.handler.prepare_paint();

                    let mut rect: RECT = mem::zeroed();
                    // TODO: use GetUpdateRgn for more conservative invalidation
                    GetUpdateRect(hwnd, &mut rect, FALSE);
                    ValidateRect(hwnd, null_mut());
                    let rect_dp = util::recti_to_rect(rect).to_dp(self.scale());
                    if rect_dp.area() != 0.0 {
                        self.invalidate_rect(rect_dp);
                    }
                    let invalid = self.take_invalid();
                    if !invalid.rects().is_empty() {
                        s.handler.rebuild_resources();
                        s.render(&self.d2d_factory, &self.dwrite_factory, &invalid);
                        if let Some(ref mut ds) = s.dxgi_state {
                            let mut dirty_rects = util::region_to_rectis(&invalid, self.scale());
                            let params = DXGI_PRESENT_PARAMETERS {
                                DirtyRectsCount: dirty_rects.len() as u32,
                                pDirtyRects: dirty_rects.as_mut_ptr(),
                                pScrollRect: null_mut(),
                                pScrollOffset: null_mut(),
                            };
                            (*ds.swap_chain).Present1(1, 0, &params);
                        }
                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                Some(0)
            },
            WM_DPICHANGED => unsafe {
                let x = HIWORD(wparam as u32) as f64 / SCALE_TARGET_DPI;
                let y = LOWORD(wparam as u32) as f64 / SCALE_TARGET_DPI;
                let scale = Scale::new(x, y);
                self.set_scale(scale);
                let rect: *mut RECT = lparam as *mut RECT;
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    (*rect).left,
                    (*rect).top,
                    (*rect).right - (*rect).left,
                    (*rect).bottom - (*rect).top,
                    SWP_NOZORDER | SWP_FRAMECHANGED | SWP_DRAWFRAME,
                );
                /*
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    if s.dxgi_state.is_some() {
                        let scale = self.scale();
                        let rt = paint::create_render_target(&self.d2d_factory, hwnd, scale);
                        s.render_target = rt.ok();
                        {
                            let rect_dp = self.area().size_dp().to_rect();
                            s.handler.rebuild_resources();
                            s.render(&self.d2d_factory, &self.dwrite_factory, &rect_dp.into());
                            self.clear_invalid();
                        }

                    }
                } else {
                    self.log_dropped_msg(hwnd, msg, wparam, lparam);
                }
                */
                Some(0)
            },
            WM_NCCALCSIZE => unsafe {
                // Workaround to get rid of caption but keeping the borders created by it.
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
                if style == 0 {
                    warn!(
                        "failed to get window style: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                    return Some(0);
                }

                if !self.has_titlebar() && (style & WS_CAPTION) != 0 {
                    let s: *mut NCCALCSIZE_PARAMS = lparam as *mut NCCALCSIZE_PARAMS;
                    if let Some(mut s) = s.as_mut() {
                        if let Some(func) = OPTIONAL_FUNCTIONS.GetSystemMetricsForDpi {
                            // This function is only supported on windows 10
                            let dpi = self.scale().x() * SCALE_TARGET_DPI;
                            // Height of the different parts that make the titlebar
                            let border = func(SM_CXPADDEDBORDER, dpi as u32);
                            let frame = func(SM_CYSIZEFRAME, dpi as u32);
                            let caption = func(SM_CYCAPTION, dpi as u32);
                            // Maximized window titlebar height is just the caption
                            if (style & WS_MAXIMIZE) != 0 {
                                s.rgrc[0].top -= (caption) as i32;
                            }
                            // Normal window titlebar height is a combination of border frame and caption
                            else {
                                s.rgrc[0].top -= (border + frame + caption) as i32;
                            }
                        }
                        // Support for windows 8.1
                        else {
                            // Note: With SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE) that we use on 8.1,
                            // Windows will not scale the titlebar area when DPI changes.
                            // Instead it is "stuck" at the DPI setting the window was created with.
                            // GetSystemMetrics() also reports values with the DPI the window was created with.
                            let border = GetSystemMetrics(SM_CXPADDEDBORDER);
                            let frame = GetSystemMetrics(SM_CYSIZEFRAME);
                            let caption = GetSystemMetrics(SM_CYCAPTION);
                            if (style & WS_MAXIMIZE) != 0 {
                                s.rgrc[0].top -= (caption) as i32;
                            } else {
                                s.rgrc[0].top -= (border + frame + caption) as i32;
                            }
                        }
                    }
                }
                None
            },
            WM_NCHITTEST => unsafe {
                let mut hit = DefWindowProcW(hwnd, msg, wparam, lparam);
                if !self.has_titlebar() {
                    let mut rect = RECT {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    };
                    if GetWindowRect(hwnd, &mut rect) == 0 {
                        warn!(
                            "failed to get window rect: {}",
                            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                        );
                    };
                    let a = HIWORD(lparam as u32) as i16 as i32 - rect.top;
                    if (a == 0) && (hit != HTTOPLEFT) && (hit != HTTOPRIGHT) && self.resizable() {
                        hit = HTTOP;
                    }
                }
                if hit != HTTOP {
                    let mouseDown = GetAsyncKeyState(VK_LBUTTON) < 0;

                    if self.with_window_state(|state| state.handle_titlebar.get()) && !mouseDown {
                        self.with_window_state(move |state| state.handle_titlebar.set(false));
                    };

                    if self.with_window_state(|state| state.handle_titlebar.get())
                        && hit == HTCLIENT
                    {
                        hit = HTCAPTION;
                    }
                }
                Some(hit)
            },
            WM_SIZE => unsafe {
                let width = LOWORD(lparam as u32) as u32;
                let height = HIWORD(lparam as u32) as u32;
                if width == 0 || height == 0 {
                    return Some(0);
                }
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let scale = self.scale();
                    let area = ScaledArea::from_px((width as f64, height as f64), scale);
                    let size_dp = area.size_dp();
                    self.set_area(area);
                    s.handler.size(size_dp);
                    let res;
                    {
                        s.render_target = None;
                        res = (*s.dxgi_state.as_mut().unwrap().swap_chain).ResizeBuffers(
                            0,
                            width,
                            height,
                            DXGI_FORMAT_UNKNOWN,
                            0,
                        );
                    }
                    if SUCCEEDED(res) {
                        s.rebuild_render_target(&self.d2d_factory, scale);
                        s.render(
                            &self.d2d_factory,
                            &self.dwrite_factory,
                            &size_dp.to_rect().into(),
                        );
                        if let Some(ref mut dxgi_state) = s.dxgi_state {
                            (*dxgi_state.swap_chain).Present(0, 0);
                        }
                        ValidateRect(hwnd, null_mut());
                    } else {
                        error!("ResizeBuffers failed: 0x{:x}", res);
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
            //TODO: WM_SYSCOMMAND
            WM_CHAR | WM_SYSCHAR | WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP
            | WM_INPUTLANGCHANGE => {
                unsafe {
                    if let Ok(mut s) = self.state.try_borrow_mut() {
                        let s = s.as_mut().unwrap();
                        if let Some(event) =
                            s.keyboard_state.process_message(hwnd, msg, wparam, lparam)
                        {
                            // If the window doesn't have a menu, then we need to suppress ALT/F10.
                            // Otherwise we will stop getting mouse events for no gain.
                            // When we do have a menu, those keys will focus the menu.
                            let handle_menu = !self.has_menu()
                                && (event.key == KbKey::Alt || event.key == KbKey::F10);
                            match event.state {
                                KeyState::Down => {
                                    if s.handler.key_down(event) || handle_menu {
                                        return Some(0);
                                    }
                                }
                                KeyState::Up => {
                                    s.handler.key_up(event);
                                    if handle_menu {
                                        return Some(0);
                                    }
                                }
                            }
                        }
                    } else {
                        self.log_dropped_msg(hwnd, msg, wparam, lparam);
                    }
                }
                None
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                // TODO: apply mouse sensitivity based on
                // SPI_GETWHEELSCROLLLINES setting.
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    let system_delta = HIWORD(wparam as u32) as i16 as f64;
                    let down_state = LOWORD(wparam as u32) as usize;
                    let mods = s.keyboard_state.get_modifiers();
                    let is_shift = mods.shift();
                    let wheel_delta = match msg {
                        WM_MOUSEWHEEL if is_shift => Vec2::new(-system_delta, 0.),
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

                    let pos = Point::new(p.x as f64, p.y as f64).to_dp(self.scale());
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

                    let pos = Point::new(x as f64, y as f64).to_dp(self.scale());
                    let mods = s.keyboard_state.get_modifiers();
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
            // Note: we handle the double-click events out of caution here, but we don't expect
            // to actually receive any, because we don't set CS_DBLCLKS on the window class style.
            // And the reason for that is that we want click counts that go above 2, so it just
            // makes a lot more sense to do the click count logic ourselves.
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
                        let down = matches!(
                            msg,
                            WM_LBUTTONDOWN
                                | WM_MBUTTONDOWN
                                | WM_RBUTTONDOWN
                                | WM_XBUTTONDOWN
                                | WM_LBUTTONDBLCLK
                                | WM_MBUTTONDBLCLK
                                | WM_RBUTTONDBLCLK
                                | WM_XBUTTONDBLCLK
                        );
                        let x = LOWORD(lparam as u32) as i16 as i32;
                        let y = HIWORD(lparam as u32) as i16 as i32;
                        let pos = Point::new(x as f64, y as f64).to_dp(self.scale());
                        let mods = s.keyboard_state.get_modifiers();
                        let buttons = get_buttons(wparam);
                        let dct = unsafe { GetDoubleClickTime() };
                        let count = if down {
                            // TODO: it may be more precise to use the timestamp from the event.
                            let this_click = Instant::now();
                            let thresh_x = unsafe { GetSystemMetrics(SM_CXDOUBLECLK) };
                            let thresh_y = unsafe { GetSystemMetrics(SM_CYDOUBLECLK) };
                            let in_box = (x - s.last_click_pos.0).abs() <= thresh_x / 2
                                && (y - s.last_click_pos.1).abs() <= thresh_y / 2;
                            let threshold = Duration::from_millis(dct as u64);
                            if this_click - s.last_click_time >= threshold || !in_box {
                                s.click_count = 0;
                            }
                            s.click_count = s.click_count.saturating_add(1);
                            s.last_click_time = this_click;
                            s.last_click_pos = (x, y);
                            s.click_count
                        } else {
                            0
                        };
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
            WM_CLOSE => {
                if let Ok(mut s) = self.state.try_borrow_mut() {
                    let s = s.as_mut().unwrap();
                    s.handler.request_close();
                    Some(0)
                } else {
                    None
                }
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
                    if let Some(min_size_dp) = s.min_size {
                        let min_area = ScaledArea::from_dp(min_size_dp, self.scale());
                        let min_size_px = min_area.size_px();
                        min_max_info.ptMinTrackSize.x = min_size_px.width as i32;
                        min_max_info.ptMinTrackSize.y = min_size_px.height as i32;
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
            DS_HANDLE_DEFERRED => {
                self.handle_deferred_queue();
                Some(0)
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
            size: None,
            min_size: None,
            position: Point::new(CW_USEDEFAULT as f64, CW_USEDEFAULT as f64),
            state: window::WindowState::RESTORED,
        }
    }

    /// This takes ownership, and is typically used with UiMain
    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = Some(size);
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

    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn set_position(&mut self, position: Point) {
        self.position = position;
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        self.state = state;
    }

    pub fn set_level(&mut self, _level: WindowLevel) {
        log::warn!("WindowBuilder::set_level  is currently unimplemented for Windows platforms.");
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

            let scale = Scale::new(1.0, 1.0);
            let mut area = ScaledArea::default();
            let (width, height) = self
                .size
                .map(|size| {
                    area = ScaledArea::from_dp(size, scale);
                    let size_px = area.size_px();
                    (size_px.width as i32, size_px.height as i32)
                })
                .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));

            let (hmenu, accels, has_menu) = match self.menu {
                Some(menu) => {
                    let accels = menu.accels();
                    (menu.into_hmenu(), accels, true)
                }
                None => (0 as HMENU, None, false),
            };

            let window = WindowState {
                hwnd: Cell::new(0 as HWND),
                scale: Cell::new(scale),
                area: Cell::new(area),
                invalid: RefCell::new(Region::EMPTY),
                has_menu: Cell::new(has_menu),
                wndproc: Box::new(wndproc),
                idle_queue: Default::default(),
                timers: Arc::new(Mutex::new(TimerSlots::new(1))),
                deferred_queue: RefCell::new(DeferredQueue::new()),
                has_titlebar: Cell::new(self.show_titlebar),
                is_resizable: Cell::new(self.resizable),
                handle_titlebar: Cell::new(false),
            };
            let win = Rc::new(window);
            let handle = WindowHandle {
                dwrite_factory,
                state: Rc::downgrade(&win),
            };

            let state = WndState {
                handler: self.handler.unwrap(),
                render_target: None,
                dxgi_state: None,
                min_size: self.min_size,
                keyboard_state: KeyboardState::new(),
                captured_mouse_buttons: MouseButtons::new(),
                has_mouse_focus: false,
                last_click_time: Instant::now(),
                last_click_pos: (0, 0),
                click_count: 0,
            };
            win.wndproc.connect(&handle, state);

            let mut dwStyle = WS_OVERLAPPEDWINDOW;
            if !self.resizable {
                dwStyle &= !(WS_THICKFRAME | WS_MAXIMIZEBOX);
            }
            if !self.show_titlebar {
                dwStyle &= !(WS_MINIMIZEBOX | WS_SYSMENU | WS_OVERLAPPED);
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
                self.position.x as i32,
                self.position.y as i32,
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

            if let Some(size) = self.size {
                // TODO: because this is deferred, it causes some flashing.
                // Investigate a proper fix that gets the window created with
                // the correct size.
                handle.set_size(size);
            }
            handle.set_window_state(self.state);

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

unsafe fn create_dxgi_state(
    present_strategy: PresentStrategy,
    hwnd: HWND,
) -> Result<Option<DxgiState>, Error> {
    let mut factory: *mut IDXGIFactory2 = null_mut();
    as_result(CreateDXGIFactory1(
        &IID_IDXGIFactory2,
        &mut factory as *mut *mut IDXGIFactory2 as *mut *mut c_void,
    ))?;
    debug!("dxgi factory pointer = {:?}", factory);
    let adapter = choose_adapter(factory);
    debug!("adapter = {:?}", adapter);

    let mut d3d11_device = D3D11Device::new_simple()?;

    let (swap_effect, bufs) = match present_strategy {
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
    let res = (*factory).CreateSwapChainForHwnd(
        d3d11_device.raw_ptr() as *mut IUnknown,
        hwnd,
        &desc,
        null_mut(),
        null_mut(),
        &mut swap_chain,
    );
    debug!("swap chain res = 0x{:x}, pointer = {:?}", res, swap_chain);

    Ok(Some(DxgiState { swap_chain }))
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
    fn get_hcursor(&self) -> HCURSOR {
        let name = match self {
            Cursor::Arrow => IDC_ARROW,
            Cursor::IBeam => IDC_IBEAM,
            Cursor::Crosshair => IDC_CROSS,
            Cursor::OpenHand => IDC_HAND,
            Cursor::NotAllowed => IDC_NO,
            Cursor::ResizeLeftRight => IDC_SIZEWE,
            Cursor::ResizeUpDown => IDC_SIZENS,
            Cursor::Custom(c) => {
                return (c.0).0;
            }
        };
        unsafe { LoadCursorW(0 as HINSTANCE, name) }
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

    pub fn request_anim_frame(&self) {
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                // With the RDW_INTERNALPAINT flag, RedrawWindow causes a WM_PAINT message, but without
                // invalidating anything. We do this because we won't know the final invalidated region
                // until after calling prepare_paint.
                if RedrawWindow(hwnd, null(), null_mut(), RDW_INTERNALPAINT) == 0 {
                    log::warn!(
                        "RedrawWindow failed: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                }
            }
        }
    }

    pub fn invalidate(&self) {
        if let Some(w) = self.state.upgrade() {
            w.invalid
                .borrow_mut()
                .set_rect(w.area.get().size_dp().to_rect());
        }
        self.request_anim_frame();
    }

    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(w) = self.state.upgrade() {
            let scale = w.scale.get();
            // We need to invalidate an integer number of pixels, but we also want to keep
            // the invalid region in display points, since that's what we need to pass
            // to WinHandler::paint.
            w.invalid
                .borrow_mut()
                .add_rect(rect.to_px(scale).expand().to_dp(scale));
        }
        self.request_anim_frame();
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

    pub fn show_titlebar(&self, show_titlebar: bool) {
        if let Some(w) = self.state.upgrade() {
            w.has_titlebar.set(show_titlebar);
            DeferredQueue::add(self.state.clone(), DeferredOp::DecorationChanged());
        }
    }

    // Sets the position of the window in virtual screen coordinates
    pub fn set_position(&self, position: Point) {
        DeferredQueue::add(
            self.state.clone(),
            DeferredOp::SetWindowSizeState(window::WindowState::RESTORED),
        );
        DeferredQueue::add(self.state.clone(), DeferredOp::SetPosition(position));
    }

    pub fn set_level(&self, _level: WindowLevel) {
        warn!("Window level unimplemented for Windows!");
    }

    // Gets the position of the window in virtual screen coordinates
    pub fn get_position(&self) -> Point {
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                if GetWindowRect(hwnd, &mut rect) == 0 {
                    warn!(
                        "failed to get window rect: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                };
                return Point::new(rect.left as f64, rect.top as f64);
            }
        }
        Point::new(0.0, 0.0)
    }

    // Sets the size of the window in DP
    pub fn set_size(&self, size: Size) {
        DeferredQueue::add(self.state.clone(), DeferredOp::SetSize(size));
    }

    // Gets the size of the window in pixels
    pub fn get_size(&self) -> Size {
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                if GetWindowRect(hwnd, &mut rect) == 0 {
                    warn!(
                        "failed to get window rect: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                };
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                return Size::new(width as f64, height as f64);
            }
        }
        Size::new(0.0, 0.0)
    }

    pub fn resizable(&self, resizable: bool) {
        if let Some(w) = self.state.upgrade() {
            w.is_resizable.set(resizable);
            DeferredQueue::add(self.state.clone(), DeferredOp::DecorationChanged());
        }
    }

    // Sets the window state.
    pub fn set_window_state(&self, state: window::WindowState) {
        DeferredQueue::add(self.state.clone(), DeferredOp::SetWindowSizeState(state));
    }

    // Gets the window state.
    pub fn get_window_state(&self) -> window::WindowState {
        // We can not store state internally because it could be modified externally.
        if let Some(w) = self.state.upgrade() {
            let hwnd = w.hwnd.get();
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
                if style == 0 {
                    warn!(
                        "failed to get window style: {}",
                        Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                    );
                }
                if (style & WS_MAXIMIZE) != 0 {
                    window::WindowState::MAXIMIZED
                } else if (style & WS_MINIMIZE) != 0 {
                    window::WindowState::MINIMIZED
                } else {
                    window::WindowState::RESTORED
                }
            }
        } else {
            window::WindowState::RESTORED
        }
    }

    // Allows windows to handle a custom titlebar like it was the default one.
    pub fn handle_titlebar(&self, val: bool) {
        if let Some(w) = self.state.upgrade() {
            w.handle_titlebar.set(val);
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
                    w.has_menu.set(true);
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
            let pos = pos.to_px(w.scale.get()).round();
            unsafe {
                let mut point = POINT {
                    x: pos.x as i32,
                    y: pos.y as i32,
                };
                ClientToScreen(hwnd, &mut point);
                if TrackPopupMenu(hmenu, TPM_LEFTALIGN, point.x, point.y, 0, hwnd, null()) == FALSE
                {
                    warn!("failed to track popup menu");
                }
            }
        }
    }

    pub fn text(&self) -> PietText {
        PietText::new(self.dwrite_factory.clone())
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
            SetCursor(cursor.get_hcursor());
        }
    }

    pub fn make_cursor(&self, cursor_desc: &CursorDesc) -> Option<Cursor> {
        if let Some(hwnd) = self.get_hwnd() {
            unsafe {
                let hdc = GetDC(hwnd);
                if hdc.is_null() {
                    return None;
                }
                defer!(ReleaseDC(null_mut(), hdc););

                let mask_dc = CreateCompatibleDC(hdc);
                if mask_dc.is_null() {
                    return None;
                }
                defer!(DeleteDC(mask_dc););

                let bmp_dc = CreateCompatibleDC(hdc);
                if bmp_dc.is_null() {
                    return None;
                }
                defer!(DeleteDC(bmp_dc););

                let width = cursor_desc.image.width();
                let height = cursor_desc.image.height();
                let mask = CreateCompatibleBitmap(hdc, width as c_int, height as c_int);
                if mask.is_null() {
                    return None;
                }
                defer!(DeleteObject(mask as _););

                let bmp = CreateCompatibleBitmap(hdc, width as c_int, height as c_int);
                if bmp.is_null() {
                    return None;
                }
                defer!(DeleteObject(bmp as _););

                let old_mask = SelectObject(mask_dc, mask as *mut c_void);
                let old_bmp = SelectObject(bmp_dc, bmp as *mut c_void);

                for (row_idx, row) in cursor_desc.image.pixel_colors().enumerate() {
                    for (col_idx, p) in row.enumerate() {
                        let (r, g, b, a) = p.as_rgba8();
                        // TODO: what's the story on partial transparency? I couldn't find documentation.
                        let mask_px = RGB(255 - a, 255 - a, 255 - a);
                        let bmp_px = RGB(r, g, b);
                        SetPixel(mask_dc, col_idx as i32, row_idx as i32, mask_px);
                        SetPixel(bmp_dc, col_idx as i32, row_idx as i32, bmp_px);
                    }
                }

                SelectObject(mask_dc, old_mask);
                SelectObject(bmp_dc, old_bmp);

                let mut icon_info = ICONINFO {
                    // 0 means it's a cursor, not an icon.
                    fIcon: 0,
                    xHotspot: cursor_desc.hot.x as DWORD,
                    yHotspot: cursor_desc.hot.y as DWORD,
                    hbmMask: mask,
                    hbmColor: bmp,
                };
                let icon = CreateIconIndirect(&mut icon_info);

                Some(Cursor::Custom(CustomCursor(Arc::new(HCursor(icon)))))
            }
        } else {
            None
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

    pub fn open_file(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        // TODO: implement this and remove open_file_sync
        None
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

    pub fn save_as(&mut self, _options: FileDialogOptions) -> Option<FileDialogToken> {
        // TODO: implement this and remove save_as_sync
        None
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

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        Ok(self
            .state
            .upgrade()
            .ok_or(ShellError::WindowDropped)?
            .scale
            .get())
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
}

impl Default for WindowHandle {
    fn default() -> Self {
        WindowHandle {
            state: Default::default(),
            dwrite_factory: DwriteFactory::new().unwrap(),
        }
    }
}
