// Copyright 2020 The druid Authors.
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
    rect: Rect, // The working Rectangle of the monitor in pixels related to Screen.size
    work_rect: Rect, // The working Rectangle of the monitor in pixels related to Screen.size excluding reserved space (the taskbar)
}

impl Monitor {
    pub fn new(primary: bool, rect: Rect, work_rect: Rect) -> Self {
        Monitor {
            primary,
            rect,
            work_rect,
        }
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
