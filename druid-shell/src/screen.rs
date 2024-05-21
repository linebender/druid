// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Module to get information about monitors

use crate::backend;
use crate::kurbo::Rect;
use std::fmt;
use std::fmt::Display;

/// Monitor struct containing data about a monitor on the system
///
/// Use [`Screen::get_monitors`] to return a `Vec<Monitor>` of all the monitors on the system
///
/// [`Screen::get_monitors`]: Screen::get_monitors
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
    /// [`monitors`]: Monitor
    pub fn get_monitors() -> Vec<Monitor> {
        backend::screen::get_monitors()
    }

    /// Returns the bounding rectangle of the total virtual screen space in pixels.
    pub fn get_display_rect() -> Rect {
        Self::get_monitors()
            .iter()
            .map(|x| x.virtual_rect())
            .fold(Rect::ZERO, |a, b| a.union(b))
    }
}
