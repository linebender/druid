// Copyright 2019 The xi-editor Authors.
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

//! The mousey bits

use crate::kurbo::Point;
use crate::{KeyModifiers, MouseButton};

/// The state of the mouse for a click, mouse-up, or move event.
///
/// In `druid`, unlike in druid_shell`, we treat the widget's coordinate
/// space and the window's coordinate space separately.
#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// The position of the mouse in the coordinate space of the receiver.
    pub pos: Point,
    /// The position of the mouse in the coordinate space of the window.
    pub window_pos: Point,
    /// Keyboard modifiers at the time of the mouse event.
    pub mods: KeyModifiers,
    /// The number of mouse clicks associated with this event. This will always
    /// be `0` for a mouse-up event.
    pub count: u32,
    /// The currently pressed button in the case of a move or click event,
    /// or the released button in the case of a mouse-up event.
    pub button: MouseButton,
}

impl From<druid_shell::MouseEvent> for MouseEvent {
    fn from(src: druid_shell::MouseEvent) -> MouseEvent {
        let druid_shell::MouseEvent {
            pos,
            mods,
            count,
            button,
        } = src;
        MouseEvent {
            pos,
            window_pos: pos,
            mods,
            count,
            button,
        }
    }
}
