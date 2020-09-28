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

use crate::kurbo::Rect;
use crate::platform;
use std::fmt;
use std::fmt::Display;

/// Monitor struct containing data about a monitor on the system
///
/// Use Screen::get_monitors() to return a Vec<Monitor> of all the monitors on the system
#[derive(Clone, Debug, PartialEq)]
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
    #[allow(dead_code)]
    pub(crate) fn new(primary: bool, rect: Rect, work_rect: Rect) -> Self {
        Monitor {
            primary,
            rect,
            work_rect,
        }
    }
    /// Returns true if the monitor is the primary monitor.
    /// The primary monitor has its origin at (0, 0) in virtual screen coordinates.
    pub fn is_primary(&self) -> bool {
        self.primary
    }
    /// Returns the monitor rectangle in virtual screen coordinates.
    pub fn virtual_rect(&self) -> Rect {
        self.rect
    }

    /// Returns the monitor working rectangle in virtual screen coordinates.
    /// The working rectangle excludes certain things like the dock and menubar on mac,
    /// and the taskbar on windows.
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

/// Information about the screen and monitors
pub struct Screen {}
impl Screen {
    /// Returns a vector of all the [`monitors`] on the system.
    ///
    /// [`monitors`]: struct.Monitor.html
    pub fn get_monitors() -> Vec<Monitor> {
        platform::screen::get_monitors()
    }

    /// Returns the bounding rectangle of the total virtual screen space in pixels.
    pub fn get_display_rect() -> Rect {
        Self::get_monitors()
            .iter()
            .map(|x| x.virtual_rect())
            .fold(Rect::ZERO, |a, b| a.union(b))
    }
}
