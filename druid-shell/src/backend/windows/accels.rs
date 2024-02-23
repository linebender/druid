// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Wrappers for Windows of Accelerate Table.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
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

static ACCEL_TABLES: Lazy<Mutex<HashMap<WindowHandle, Arc<AccelTable>>>> =
    Lazy::new(|| Mutex::new(HashMap::default()));

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
