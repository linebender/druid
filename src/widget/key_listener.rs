// Copyright 2018 The xi-editor Authors.
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

//! Widget for forwarding key events to a listener.

use druid_shell::keycodes::M_ALT;
use druid_shell::window::{KeyEvent, KeyModifiers};

use crate::widget::Widget;
use crate::{HandlerCtx, Id, Ui};

pub struct KeyListener;

// TODO: This is Windows specific
const VK_MENU: u16 = 0x12;
const VK_F4: u16 = 0x73;
const VK_F10: u16 = 0x79;

impl KeyListener {
    pub fn new() -> Self {
        KeyListener
    }

    pub fn ui(self, child: Id, ctx: &mut Ui) -> Id {
        ctx.add(self, &[child])
    }
}

impl Widget for KeyListener {
    fn key(&mut self, event: &KeyEvent, ctx: &mut HandlerCtx) -> bool {
        // TODO: maybe some configuration of which keys are handled. Right
        // now we handle everything except a few keys.
        if (event.virtual_key == VK_F4 && event.modifiers.alt)
            || event.virtual_key == VK_F10
            || event.virtual_key == VK_MENU
        {
            return false;
        }
        ctx.send_event(event.clone());
        true
    }
}
