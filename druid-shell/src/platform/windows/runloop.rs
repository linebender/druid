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
use std::sync::{Arc, Mutex};
use winapi::um::winbase::*;
use winapi::um::winnt::*;
use winapi::um::winuser::*;

use super::accels;

// TODO: remove this, it's been obsoleted by IdleHandle
#[derive(Clone, Default)]
pub struct RunLoopHandle(Arc<Mutex<RunLoopState>>);

#[derive(Default)]
struct RunLoopState {
    listeners: Vec<Listener>,
}

// It's only safe to add listeners from the same thread as the runloop.
unsafe impl Send for Listener {}
struct Listener {
    h: HANDLE,
    callback: Box<dyn FnMut()>,
}

pub struct RunLoop {
    handle: RunLoopHandle,
}

impl RunLoop {
    pub fn new() -> RunLoop {
        RunLoop {
            handle: Default::default(),
        }
    }

    /// Get a handle to the run loop state so a client can add listeners,
    /// etc.
    pub fn get_handle(&self) -> RunLoopHandle {
        self.handle.clone()
    }

    // WAIT_OBJECT_0 is defined as 0, so >= is technically meaningless
    // but communicates intent
    #[allow(clippy::absurd_extreme_comparisons)]
    pub fn run(&mut self) {
        unsafe {
            loop {
                let mut handles = Vec::new();
                for listener in &self.handle.0.lock().unwrap().listeners {
                    handles.push(listener.h);
                }
                let len = handles.len() as u32;
                let res =
                    MsgWaitForMultipleObjectsEx(len, handles.as_ptr(), INFINITE, QS_ALLEVENTS, 0);

                // Prioritize rpc results above windows messages
                if res >= WAIT_OBJECT_0 && res < WAIT_OBJECT_0 + len {
                    let ix = (res - WAIT_OBJECT_0) as usize;
                    (&mut self.handle.0.lock().unwrap().listeners[ix].callback)();
                }

                // Handle windows messages
                loop {
                    let mut msg = mem::MaybeUninit::uninit();
                    // Note: we could use PM_REMOVE here and avoid the GetMessage below
                    let res = PeekMessageW(msg.as_mut_ptr(), null_mut(), 0, 0, PM_NOREMOVE);
                    if res == 0 {
                        break;
                    }
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
}

impl RunLoopHandle {
    /// Add a listener for a Windows handle.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the handle is valid, and that this function
    /// is only called from the main thread.
    pub unsafe fn add_handler<F>(&self, h: HANDLE, callback: F)
    where
        F: FnMut() + 'static,
    {
        let listener = Listener {
            h,
            callback: Box::new(callback),
        };
        self.0.lock().unwrap().listeners.push(listener);
    }
}
