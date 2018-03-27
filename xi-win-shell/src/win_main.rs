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

//! Windows main loop.

use std::mem;
use std::ptr::null_mut;
use winapi::um::winbase::*;
use winapi::um::winuser::*;

pub struct RunLoop {
}

impl RunLoop {
    pub fn new() -> RunLoop {
        RunLoop {}
    }

    pub fn run(&mut self) {
    // let optional_functions = util::load_optional_functions();

        unsafe {
            /*
            if let Some(func) = optional_functions.SetProcessDpiAwareness {
                // This function is only supported on windows 10
                func(PROCESS_SYSTEM_DPI_AWARE); // TODO: per monitor (much harder)
            }

            let (xi_peer, rx, semaphore) = start_xi_thread();
            let (hwnd, main_win) = create_main(&optional_functions, xi_peer).unwrap();
            ShowWindow(hwnd, SW_SHOWNORMAL);
            UpdateWindow(hwnd);
            */

            loop {
                // let handles = [semaphore.get_handle()];
                let handles = [];
                let _res = MsgWaitForMultipleObjectsEx(
                    handles.len() as u32,
                    handles.as_ptr(),
                    INFINITE,
                    QS_ALLEVENTS,
                    0
                );

                // Handle windows messages
                loop {
                    let mut msg = mem::uninitialized();
                    // Note: we could use PM_REMOVE here and avoid the GetMessage below
                    let res = PeekMessageW(&mut msg, null_mut(), 0, 0, PM_NOREMOVE);
                    if res == 0 {
                        break;
                    }
                    let res = GetMessageW(&mut msg, null_mut(), 0, 0);
                    if res <= 0 {
                        return;
                    }
                    TranslateMessage(&mut msg);
                    DispatchMessageW(&mut msg);
                }

                /*
                // Handle xi events
                loop {
                    match rx.try_recv() {
                        Ok(v) => main_win.handle_cmd(&v),
                        Err(TryRecvError::Disconnected) => {
                            println!("core disconnected");
                            break;
                        }
                        Err(TryRecvError::Empty) => break,
                    }
                }
                */
            }
        }
    }
}

pub fn request_quit() {
    unsafe {
        PostQuitMessage(0);
    }
}