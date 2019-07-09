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

use druid_shell::keyboard::KeyEvent;
use druid_shell::window::MouseEvent;

#[derive(Debug, Clone)]
pub enum Event {
    MouseDown(MouseEvent),
    MouseUp(MouseEvent),
    MouseMoved(MouseEvent),
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    HotChanged(bool),
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
