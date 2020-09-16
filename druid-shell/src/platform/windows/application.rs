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

//! Windows implementation of features at the application scope.

use std::cell::RefCell;
use std::collections::HashSet;
use std::mem;
use std::ptr;
use std::rc::Rc;

use winapi::shared::minwindef::{FALSE, HINSTANCE};
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, HCURSOR, HWND};
use winapi::shared::winerror::HRESULT_FROM_WIN32;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::shellscalingapi::PROCESS_PER_MONITOR_DPI_AWARE;
use winapi::um::winuser::{
    DispatchMessageW, GetAncestor, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage,
    RegisterClassW, SendMessageW, TranslateAcceleratorW, TranslateMessage, GA_ROOT,
    IDI_APPLICATION, MSG, WNDCLASSW,
};

use crate::application::AppHandler;

use super::accels;
use super::clipboard::Clipboard;
use super::error::Error;
use super::util::{self, ToWide, CLASS_NAME, OPTIONAL_FUNCTIONS};
use super::window::{self, DS_HANDLE_DEFERRED, DS_REQUEST_DESTROY};

#[derive(Clone)]
pub(crate) struct Application {
    state: Rc<RefCell<State>>,
}

struct State {
    quitting: bool,
    windows: HashSet<HWND>,
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        Application::init()?;
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            windows: HashSet::new(),
        }));
        Ok(Application { state })
    }

    /// Initialize the app. At the moment, this is mostly needed for hi-dpi.
    fn init() -> Result<(), Error> {
        // TODO: Report back an error instead of panicking
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
        unsafe {
            let class_name = CLASS_NAME.to_wide();
            let icon = LoadIconW(0 as HINSTANCE, IDI_APPLICATION);
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
            let class_atom = RegisterClassW(&wnd);
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
            // Handle windows messages
            loop {
                if let Ok(state) = self.state.try_borrow() {
                    for hwnd in &state.windows {
                        SendMessageW(*hwnd, DS_HANDLE_DEFERRED, 0, 0);
                    }
                }
                let mut msg = mem::MaybeUninit::uninit();
                let res = GetMessageW(msg.as_mut_ptr(), ptr::null_mut(), 0, 0);
                if res <= 0 {
                    if res == -1 {
                        log::error!(
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
                            log::warn!(
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
            log::warn!("Application state already borrowed");
        }
    }

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}
