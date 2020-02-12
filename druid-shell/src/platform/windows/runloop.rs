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

//! Windows main loop.

use std::mem;
use std::ptr::null_mut;
use winapi::um::winuser::*;

use super::accels;

pub struct RunLoop;

impl RunLoop {
    pub fn new() -> RunLoop {
        RunLoop
    }

    // WAIT_OBJECT_0 is defined as 0, so >= is technically meaningless
    // but communicates intent
    #[allow(clippy::absurd_extreme_comparisons)]
    pub fn run(&mut self) {
        unsafe {
            // Handle windows messages
            loop {
                let mut msg = mem::MaybeUninit::uninit();
                let res = GetMessageW(msg.as_mut_ptr(), null_mut(), 0, 0);
                if res <= 0 {
                    return;
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
}
