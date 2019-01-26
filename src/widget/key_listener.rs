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

use druid_win_shell::window::M_ALT;
use winapi::um::winuser::*;

use widget::Widget;
use {HandlerCtx, Id, KeyEvent, KeyVariant, Ui};

pub struct KeyListener;

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
        if let KeyVariant::Vkey(vk) = event.key {
            if (vk == VK_F4 && (event.mods & M_ALT) != 0) || vk == VK_F10 || vk == VK_MENU {
                return false;
            }
        }
        ctx.send_event(event.clone());
        true
    }
}
