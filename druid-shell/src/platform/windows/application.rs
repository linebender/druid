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

use winapi::shared::minwindef::HINSTANCE;
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::HCURSOR;
use winapi::um::shellscalingapi::PROCESS_SYSTEM_DPI_AWARE;
use winapi::um::wingdi::CreateSolidBrush;
use winapi::um::winuser::{LoadIconW, PostQuitMessage, RegisterClassW, IDI_APPLICATION, WNDCLASSW};

use super::clipboard::Clipboard;
use super::util::{self, ToWide, CLASS_NAME, OPTIONAL_FUNCTIONS};
use super::window::win_proc_dispatch;

pub struct Application;

impl Application {
    /// Initialize the app. At the moment, this is mostly needed for hi-dpi.
    pub fn init() {
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
                lpfnWndProc: Some(win_proc_dispatch),
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
        unsafe {
            PostQuitMessage(0);
        }
    }

    pub fn clipboard() -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}
