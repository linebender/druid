// Copyright 2020 The Druid Authors.
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

//! Windows Monitors and Screen information.

use super::error::Error;
use log::warn;
use std::mem::size_of;
use std::ptr::null_mut;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winuser::*;

use crate::kurbo::Rect;
use crate::screen::Monitor;

unsafe extern "system" fn monitorenumproc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: LPRECT,
    _lparam: LPARAM,
) -> BOOL {
    let rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        rcMonitor: rect,
        rcWork: rect,
        dwFlags: 0,
    };
    if GetMonitorInfoW(hmonitor, &mut info) == 0 {
        warn!(
            "failed to get Monitor Info: {}",
            Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
        );
    };
    let primary = info.dwFlags == MONITORINFOF_PRIMARY;
    let rect = Rect::new(
        info.rcMonitor.left as f64,
        info.rcMonitor.top as f64,
        info.rcMonitor.right as f64,
        info.rcMonitor.bottom as f64,
    );
    let work_rect = Rect::new(
        info.rcWork.left as f64,
        info.rcWork.top as f64,
        info.rcWork.right as f64,
        info.rcWork.bottom as f64,
    );
    let monitors = _lparam as *mut Vec<Monitor>;
    (*monitors).push(Monitor::new(primary, rect, work_rect));
    TRUE
}

pub(crate) fn get_monitors() -> Vec<Monitor> {
    unsafe {
        let monitors = Vec::<Monitor>::new();
        let ptr = &monitors as *const Vec<Monitor>;
        if EnumDisplayMonitors(null_mut(), null_mut(), Some(monitorenumproc), ptr as isize) == 0 {
            warn!(
                "Failed to Enumerate Display Monitors: {}",
                Error::Hr(HRESULT_FROM_WIN32(GetLastError()))
            );
        };
        monitors
    }
}
