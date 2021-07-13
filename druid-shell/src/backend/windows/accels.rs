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

//! Wrappers for Windows of Accelerate Table.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use winapi::ctypes::c_int;
use winapi::shared::windef::*;
use winapi::um::winuser::*;

// NOTE:
// https://docs.microsoft.com/en-us/windows/win32/wsw/thread-safety
// All handles you obtain from functions in Kernel32 are thread-safe,
// unless the MSDN Library article for the function explicitly mentions it is not.

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct WindowHandle(HWND);
unsafe impl Send for WindowHandle {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct AccelHandle(HACCEL);
unsafe impl Send for AccelHandle {}
unsafe impl Sync for AccelHandle {}

lazy_static! {
    static ref ACCEL_TABLES: Mutex<HashMap<WindowHandle, Arc<AccelTable>>> =
        Mutex::new(HashMap::default());
}

/// A Accelerators Table for Windows
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct AccelTable {
    accel: AccelHandle,
}

impl AccelTable {
    fn new(accel: &[ACCEL]) -> AccelTable {
        let accel =
            unsafe { CreateAcceleratorTableW(accel as *const _ as *mut _, accel.len() as c_int) };
        AccelTable {
            accel: AccelHandle(accel),
        }
    }

    pub(crate) fn handle(&self) -> HACCEL {
        self.accel.0
    }
}

pub(crate) fn register_accel(hwnd: HWND, accel: &[ACCEL]) {
    let mut table = ACCEL_TABLES.lock().unwrap();
    table.insert(WindowHandle(hwnd), Arc::new(AccelTable::new(accel)));
}

impl Drop for AccelTable {
    fn drop(&mut self) {
        unsafe {
            DestroyAcceleratorTable(self.accel.0);
        }
    }
}

pub(crate) fn find_accels(hwnd: HWND) -> Option<Arc<AccelTable>> {
    let table = ACCEL_TABLES.lock().unwrap();
    table.get(&WindowHandle(hwnd)).cloned()
}
