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

//! Module to get information about monitors

use crate::kurbo::{Rect, Size};
use crate::platform;
use std::fmt;
use std::fmt::Display;

#[derive(Clone)]
pub struct Monitor {
    primary: bool,
    rect: Rect,
    // TODO: Work area, cross_platform
    // https://developer.apple.com/documentation/appkit/nsscreen/1388369-visibleframe
    // https://developer.gnome.org/gdk3/stable/GdkMonitor.html#gdk-monitor-get-workarea
    // https://docs.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfo
    // Unsure about x11
    work_rect: Rect,
}

impl Monitor {
    pub(crate) fn new(primary: bool, rect: Rect, work_rect: Rect) -> Self {
        Monitor {
            primary,
            rect,
            work_rect,
        }
    }
    /// Returns true if the monitor is the primary monitor
    /// a primary monitor has its top-left corner at (0,0)
    /// in virtual screen coordinates.
    pub fn is_primary(&self) -> bool {
        self.primary
    }
    /// Returns a RECT of the monitor rectangle in virtual screen coordinates
    /// meaning that it contains the monitors resolution
    /// oriented around the origin point: (0,0) being the top-left corner
    /// of the primary monitor, in pixels.
    pub fn virtual_rect(&self) -> Rect {
        self.rect
    }

    /// Returns a RECT of the monitor working rectangle in virtual screen coordinates
    /// The RECT excludes area occupied by things like the dock,menubar (mac). taskbar (windows)
    pub fn virtual_work_rect(&self) -> Rect {
        self.work_rect
    }
}

impl Display for Monitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.primary {
            write!(f, "Primary ")?;
        } else {
            write!(f, "Secondary ")?;
        }
        write!(
            f,
            "({}, {})({}, {})",
            self.rect.x0, self.rect.x1, self.rect.y0, self.rect.y1
        )?;
        Ok(())
    }
}

pub struct Screen {}
impl Screen {
    pub fn get_monitors() -> Vec<Monitor> {
        platform::screen::get_monitors()
    }

    pub fn get_display_size() -> Size {
        platform::screen::get_display_size()
    }
}
