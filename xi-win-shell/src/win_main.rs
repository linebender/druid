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

use std::cell::RefCell;
use std::mem;
use std::ptr::null_mut;
use std::rc::Rc;
use winapi::shared::minwindef::UINT;
use winapi::um::winbase::*;
use winapi::um::winnt::*;
use winapi::um::winuser::*;

use window::WindowHandle;

/// Message indicating there are idle tasks to run.
pub(crate) const XI_RUN_IDLE: UINT = WM_USER;

#[derive(Clone, Default)]
pub struct RunLoopHandle(Rc<RefCell<RunLoopState>>);

#[derive(Default)]
struct RunLoopState {
    listeners: Vec<Listener>,
    idle: Vec<Box<IdleCallback>>,
}

struct Listener {
    h: HANDLE,
    callback: Box<FnMut()>,
}

pub struct RunLoop {
    handle: RunLoopHandle,
}

pub trait IdleCallback {
    fn call(self: Box<Self>);
}

impl<F: FnOnce()> IdleCallback for F {
    fn call(self: Box<F>) {
        (*self)()
    }
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

    pub fn run(&mut self) {

        unsafe {
            loop {
                let mut handles = Vec::new();
                for listener in &self.handle.0.borrow().listeners {
                    handles.push(listener.h);
                }
                let len = handles.len() as u32;
                let res = MsgWaitForMultipleObjectsEx(
                    len,
                    handles.as_ptr(),
                    INFINITE,
                    QS_ALLEVENTS,
                    0
                );

                // Prioritize rpc results above windows messages
                if res >= WAIT_OBJECT_0 && res < WAIT_OBJECT_0 + len {
                    let ix = (res - WAIT_OBJECT_0) as usize;
                    (&mut self.handle.0.borrow_mut().listeners[ix].callback)();
                }

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
            }
        }
    }
}

/// Request to quit the application, exiting the runloop.
pub fn request_quit() {
    unsafe {
        PostQuitMessage(0);
    }
}

impl RunLoopHandle {
    /// Add a listener for a Windows handle. Considered unsafe because the
    /// handle must be valid.
    pub unsafe fn add_handler<F>(&self, h: HANDLE, callback: F)
        where F: FnMut() + 'static
    {
        let listener = Listener {
            h,
            callback: Box::new(callback),
        };
        self.0.borrow_mut().listeners.push(listener);
    }

    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the specified window's
    /// wndproc, which means it won't be scheduled if the window is closed.
    pub fn add_idle<F>(&self, window: &WindowHandle, callback: F) where F: FnOnce() + 'static {
        let mut state = self.0.borrow_mut();
        if state.idle.is_empty() {
            if let Some(hwnd) = window.get_hwnd() {
                unsafe {
                    PostMessageW(hwnd, XI_RUN_IDLE, 0, 0);
                }
            }
        }
        state.idle.push(Box::new(callback));
    }

    /// Run the idle tasks.
    pub(crate) fn run_idle(&self) {
        let idles = mem::replace(&mut self.0.borrow_mut().idle, Vec::new());
        for callback in idles {
            callback.call();
        }
    }
}
