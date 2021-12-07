// Copyright 2019 The Druid Authors.
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

#![allow(unused)]
use super::window::WindowHandle;
use crate::common_util::strip_access_key;
use crate::hotkey::{HotKey, RawMods};
use crate::keyboard::{KbKey, Modifiers};

#[derive(Default, Debug)]
pub struct Menu;

#[derive(Debug)]
struct MenuItem;

impl Menu {
    pub fn new() -> Menu {
        Menu
    }

    pub fn new_for_popup() -> Menu {
        Menu
    }

    pub fn add_dropdown(&mut self, menu: Menu, text: &str, _enabled: bool) {}

    pub fn add_item(
        &mut self,
        id: u32,
        text: &str,
        key: Option<&HotKey>,
        enabled: bool,
        _selected: bool,
    ) {
    }

    pub fn add_separator(&mut self) {}
}
