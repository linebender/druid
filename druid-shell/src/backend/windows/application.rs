// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Windows implementation of features at the application scope.

use std::cell::RefCell;
use std::collections::HashSet;
use std::mem;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use winapi::shared::minwindef::{FALSE, HINSTANCE};
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, HCURSOR, HWND};
use winapi::shared::winerror::HRESULT_FROM_WIN32;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellscalingapi::PROCESS_PER_MONITOR_DPI_AWARE;
use winapi::um::winnls::GetUserDefaultLocaleName;
use winapi::um::winnt::LOCALE_NAME_MAX_LENGTH;
use winapi::um::winuser::{
    DispatchMessageW, GetAncestor, GetMessageW, LoadIconW, PeekMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, TranslateAcceleratorW, TranslateMessage, GA_ROOT,
    MAKEINTRESOURCEW, MSG, PM_NOREMOVE, WM_TIMER, WNDCLASSW,
};

use piet_common::D2DLoadedFonts;

use crate::application::AppHandler;

use super::accels;
use super::clipboard::Clipboard;
use super::error::Error;
use super::util::{self, FromWide, ToWide, CLASS_NAME, OPTIONAL_FUNCTIONS};
use super::window::{self, DS_REQUEST_DESTROY};

#[derive(Clone)]
pub(crate) struct Application {
    state: Rc<RefCell<State>>,
    pub(crate) fonts: D2DLoadedFonts,
}

struct State {
    quitting: bool,
    windows: HashSet<HWND>,
}

/// Used to ensure the window class is registered only once per process.
static WINDOW_CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

impl Application {
    pub fn new() -> Result<Application, Error> {
        Application::init()?;
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            windows: HashSet::new(),
        }));
        let fonts = D2DLoadedFonts::default();
        Ok(Application { state, fonts })
    }

    /// Initialize the app. At the moment, this is mostly needed for hi-dpi.
    // TODO: Report back an error instead of panicking
    #[allow(clippy::unnecessary_wraps)]
    fn init() -> Result<(), Error> {
        util::attach_console();
        if let Some(func) = OPTIONAL_FUNCTIONS.SetProcessDpiAwarenessContext {
            // This function is only supported on windows 10
            unsafe {
                func(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            }
        } else if let Some(func) = OPTIONAL_FUNCTIONS.SetProcessDpiAwareness {
            unsafe {
                func(PROCESS_PER_MONITOR_DPI_AWARE);
            }
        }
        if WINDOW_CLASS_REGISTERED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let class_name = CLASS_NAME.to_wide();
            let icon = unsafe { LoadIconW(GetModuleHandleW(0 as LPCWSTR), MAKEINTRESOURCEW(1)) };
            let wnd = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(window::win_proc_dispatch),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as HINSTANCE,
                hIcon: icon,
                hCursor: 0 as HCURSOR,
                hbrBackground: ptr::null_mut(), // We control all the painting
                lpszMenuName: 0 as LPCWSTR,
                lpszClassName: class_name.as_ptr(),
            };
            let class_atom = unsafe { RegisterClassW(&wnd) };
            if class_atom == 0 {
                panic!("Error registering class");
            }
        }
        Ok(())
    }

    pub fn add_window(&self, hwnd: HWND) -> bool {
        self.state.borrow_mut().windows.insert(hwnd)
    }

    pub fn remove_window(&self, hwnd: HWND) -> bool {
        self.state.borrow_mut().windows.remove(&hwnd)
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        unsafe {
            // Handle windows messages.
            //
            // NOTE: Code here will not run when we aren't in charge of the message loop. That
            // will include when moving or resizing the window, and when showing modal dialogs.
            loop {
                let mut msg = mem::MaybeUninit::uninit();

                // Timer messages have a low priority and tend to get delayed. Peeking for them
                // helps for some reason; see
                // https://devblogs.microsoft.com/oldnewthing/20191108-00/?p=103080
                PeekMessageW(
                    msg.as_mut_ptr(),
                    ptr::null_mut(),
                    WM_TIMER,
                    WM_TIMER,
                    PM_NOREMOVE,
                );

                let res = GetMessageW(msg.as_mut_ptr(), ptr::null_mut(), 0, 0);
                if res <= 0 {
                    if res == -1 {
                        tracing::error!(
                            "GetMessageW failed: {}",
                            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                        );
                    }
                    break;
                }
                let mut msg: MSG = msg.assume_init();
                let accels = accels::find_accels(GetAncestor(msg.hwnd, GA_ROOT));
                let translated = accels.map_or(false, |it| {
                    TranslateAcceleratorW(msg.hwnd, it.handle(), &mut msg) != 0
                });
                if !translated {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    pub fn quit(&self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            if !state.quitting {
                state.quitting = true;
                unsafe {
                    // We want to queue up the destruction of all our windows.
                    // Failure to do so will lead to resource leaks
                    // and an eventual error code exit for the process.
                    for hwnd in &state.windows {
                        if PostMessageW(*hwnd, DS_REQUEST_DESTROY, 0, 0) == FALSE {
                            tracing::warn!(
                                "PostMessageW DS_REQUEST_DESTROY failed: {}",
                                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                            );
                        }
                    }
                    // PostQuitMessage sets a quit request flag in the OS.
                    // The actual WM_QUIT message is queued but won't be sent
                    // until all other important events have been handled.
                    PostQuitMessage(0);
                }
            }
        } else {
            tracing::warn!("Application state already borrowed");
        }
    }

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        let mut buf = [0u16; LOCALE_NAME_MAX_LENGTH];
        let len_with_null =
            unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as _) as usize };
        let locale = if len_with_null > 0 {
            buf.get(..len_with_null - 1).and_then(FromWide::to_string)
        } else {
            None
        };
        locale.unwrap_or_else(|| {
            tracing::warn!("Failed to get user locale");
            "en-US".into()
        })
    }
}
