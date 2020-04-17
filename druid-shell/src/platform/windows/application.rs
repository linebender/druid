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

//! Windows implementation of features at the application scope.

use std::cell::RefCell;
use std::collections::HashSet;
use std::mem;
use std::ptr;
use std::rc::Rc;

use winapi::shared::minwindef::{FALSE, HINSTANCE};
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::{HCURSOR, HWND};
use winapi::shared::winerror::HRESULT_FROM_WIN32;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::shellscalingapi::PROCESS_SYSTEM_DPI_AWARE;
use winapi::um::wingdi::CreateSolidBrush;
use winapi::um::winuser::{
    DispatchMessageW, GetAncestor, GetMessageW, LoadIconW, PostMessageW, RegisterClassW,
    TranslateAcceleratorW, TranslateMessage, GA_ROOT, IDI_APPLICATION, MSG, WNDCLASSW,
};

use crate::application::AppHandler;

use super::accels;
use super::clipboard::Clipboard;
use super::error::Error;
use super::util::{self, ToWide, CLASS_NAME, OPTIONAL_FUNCTIONS};
use super::window::{self, DS_REQUEST_QUIT};

thread_local! {
    static GLOBAL_STATE: RefCell<Option<AppState>> = RefCell::new(None);
}

#[derive(Clone)]
pub struct AppState {
    state: Rc<RefCell<State>>,
}

struct State {
    quitting: bool,
    app_hwnd: Option<HWND>,
    windows: HashSet<HWND>,
}

pub struct Application;

impl AppState {
    pub(crate) fn new() -> AppState {
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            app_hwnd: None,
            windows: HashSet::new(),
        }));
        AppState { state }
    }

    pub(crate) fn quitting(&self) -> bool {
        self.state.borrow().quitting
    }

    pub(crate) fn set_quitting(&self, quitting: bool) {
        self.state.borrow_mut().quitting = quitting;
    }

    pub(crate) fn app_hwnd(&self) -> Option<HWND> {
        self.state.borrow().app_hwnd
    }

    pub(crate) fn set_app_hwnd(&self, app_hwnd: Option<HWND>) {
        self.state.borrow_mut().app_hwnd = app_hwnd;
    }

    /// Returns a set of `HWND` for all the current normal windows.
    ///
    /// The returned set should be treated with extremely limited lifetime.
    /// The window handles it contains can become stale quickly.
    #[allow(clippy::mutable_key_type)]
    pub(crate) unsafe fn windows(&self) -> HashSet<HWND> {
        self.state.borrow().windows.clone()
    }

    pub(crate) fn add_window(&self, hwnd: HWND) -> bool {
        self.state.borrow_mut().windows.insert(hwnd)
    }

    pub(crate) fn remove_window(&self, hwnd: HWND) -> bool {
        self.state.borrow_mut().windows.remove(&hwnd)
    }
}

impl Application {
    pub fn new(state: AppState, _handler: Option<Box<dyn AppHandler>>) -> Application {
        util::claim_main_thread();
        GLOBAL_STATE.with(|global_state| {
            *global_state.borrow_mut() = Some(state.clone());
        });
        Application::init();
        window::build_app_window(state).expect("Failed to build main message window");
        Application
    }

    pub fn run(&mut self) {
        unsafe {
            // Handle windows messages
            loop {
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

    /// Initialize the app. At the moment, this is mostly needed for hi-dpi.
    fn init() {
        util::assert_main_thread();
        util::attach_console();
        if let Some(func) = OPTIONAL_FUNCTIONS.SetProcessDpiAwareness {
            // This function is only supported on windows 10
            unsafe {
                func(PROCESS_SYSTEM_DPI_AWARE); // TODO: per monitor (much harder)
            }
        }

        unsafe {
            let class_name = CLASS_NAME.to_wide();
            let icon = LoadIconW(0 as HINSTANCE, IDI_APPLICATION);
            let brush = CreateSolidBrush(0xff_ff_ff);
            let wnd = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(window::win_proc_dispatch),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: 0 as HINSTANCE,
                hIcon: icon,
                hCursor: 0 as HCURSOR,
                hbrBackground: brush,
                lpszMenuName: 0 as LPCWSTR,
                lpszClassName: class_name.as_ptr(),
            };
            let class_atom = RegisterClassW(&wnd);
            if class_atom == 0 {
                panic!("Error registering class");
            }
        }
    }

    pub fn quit() {
        util::assert_main_thread();
        GLOBAL_STATE.with(|global_state| {
            if let Some(global_state) = global_state.borrow().as_ref() {
                if let Some(app_hwnd) = global_state.app_hwnd() {
                    unsafe {
                        if PostMessageW(app_hwnd, DS_REQUEST_QUIT, 0, 0) == FALSE {
                            log::error!(
                                "PostMessageW failed: {}",
                                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
                            );
                        }
                    }
                }
            }
        });
    }

    pub fn clipboard() -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        GLOBAL_STATE.with(|global_state| {
            *global_state.borrow_mut() = None;
        });
        util::release_main_thread();
    }
}
