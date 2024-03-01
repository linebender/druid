// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

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

    pub fn add_dropdown(&mut self, menu: Menu, text: &str, _enabled: bool) {
        tracing::warn!("unimplemented");
    }

    pub fn add_item(
        &mut self,
        _id: u32,
        _text: &str,
        _key: Option<&HotKey>,
        _selected: Option<bool>,
        _enabled: bool,
    ) {
        tracing::warn!("unimplemented");
    }

    pub fn add_separator(&mut self) {
        tracing::warn!("unimplemented");
    }
}
