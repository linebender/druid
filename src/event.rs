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

//! Events.

use crate::kurbo::Vec2;

use druid_shell::keyboard::{KeyEvent, KeyModifiers};
use druid_shell::window::MouseEvent;

#[derive(Debug, Clone)]
pub enum Event {
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseMoved(MouseEvent),
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    Wheel(WheelEvent),
    HotChanged(bool),
}

#[derive(Debug, Clone)]
pub struct WheelEvent {
    /// The wheel movement.
    ///
    /// The polarity is the amount to be added to the scroll position,
    /// in other words the opposite of the direction the content should
    /// move on scrolling. This polarity is consistent with the
    /// deltaX and deltaY values in a web [WheelEvent].
    ///
    /// [WheelEvent]: https://w3c.github.io/uievents/#event-type-wheel
    pub delta: Vec2,
    /// The keyboard modifiers at the time of the event.
    pub mods: KeyModifiers,
}

impl Event {
    /// Whether the event should be propagated from parent to children.
    pub(crate) fn recurse(&self) -> bool {
        match self {
            Event::HotChanged(_) => false,
            _ => true,
        }
    }
}
