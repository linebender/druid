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

//! Map input to `EditAction`s

use super::Movement;
use crate::{HotKey, KbKey, KeyEvent, Modifiers, SysMods};

// This following enumerations are heavily inspired by xi-editors enumerations found at
// https://github.com/xi-editor/xi-editor/blob/e2589974fc4050beb33af82481aa71b258358e48/rust/core-lib/src/edit_types.rs
// This is done with the goal of eventually being able to easily switch
// to a xi-based implementation of our Events.

/// An enum that represents actions in a text buffer.
#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub enum EditAction {
    Move(Movement),
    ModifySelection(Movement),
    SelectAll,
    Click(MouseAction),
    Drag(MouseAction),
    Delete,
    Backspace,
    JumpDelete(Movement),
    JumpBackspace(Movement),
    Insert(String),
    Paste(String),
}

/// Extra information related to mouse actions
#[derive(PartialEq, Debug, Clone)]
#[allow(missing_docs)]
pub struct MouseAction {
    pub row: usize,
    pub column: usize,
    pub mods: Modifiers,
}

/// A trait for types that map keyboard events to possible edit actions.
pub trait TextInput {
    /// Handle a key event and return an edit action to be executed
    /// for the key event
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction>;
}

/// Handles key events and returns actions that are applicable to
/// single line textboxes
#[derive(Default)]
pub struct BasicTextInput {}

impl BasicTextInput {
    /// Create a new `BasicTextInput`.
    pub fn new() -> Self {
        Self {}
    }
}

impl TextInput for BasicTextInput {
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        let action = match event {
            // Select left word (Shift+Ctrl+ArrowLeft || Shift+Cmd+ArrowLeft)
            k_e if (HotKey::new(SysMods::CmdShift, KbKey::ArrowLeft)).matches(k_e) => {
                EditAction::ModifySelection(Movement::LeftWord)
            }
            // Select right word (Shift+Ctrl+ArrowRight || Shift+Cmd+ArrowRight)
            k_e if (HotKey::new(SysMods::CmdShift, KbKey::ArrowRight)).matches(k_e) => {
                EditAction::ModifySelection(Movement::RightWord)
            }
            // Select to home (Shift+Home)
            k_e if (HotKey::new(SysMods::Shift, KbKey::Home)).matches(k_e) => {
                EditAction::ModifySelection(Movement::PrecedingLineBreak)
            }
            // Select to end (Shift+End)
            k_e if (HotKey::new(SysMods::Shift, KbKey::End)).matches(k_e) => {
                EditAction::ModifySelection(Movement::NextLineBreak)
            }
            // Select left (Shift+ArrowLeft)
            k_e if (HotKey::new(SysMods::Shift, KbKey::ArrowLeft)).matches(k_e) => {
                EditAction::ModifySelection(Movement::Left)
            }
            // Select right (Shift+ArrowRight)
            k_e if (HotKey::new(SysMods::Shift, KbKey::ArrowRight)).matches(k_e) => {
                EditAction::ModifySelection(Movement::Right)
            }
            // Select all (Ctrl+A || Cmd+A)
            k_e if (HotKey::new(SysMods::Cmd, "a")).matches(k_e) => EditAction::SelectAll,
            // Left word (Ctrl+ArrowLeft || Cmd+ArrowLeft)
            k_e if (HotKey::new(SysMods::Cmd, KbKey::ArrowLeft)).matches(k_e) => {
                EditAction::Move(Movement::LeftWord)
            }
            // Right word (Ctrl+ArrowRight || Cmd+ArrowRight)
            k_e if (HotKey::new(SysMods::Cmd, KbKey::ArrowRight)).matches(k_e) => {
                EditAction::Move(Movement::RightWord)
            }
            // Move left (ArrowLeft)
            k_e if (HotKey::new(None, KbKey::ArrowLeft)).matches(k_e) => {
                EditAction::Move(Movement::Left)
            }
            // Move right (ArrowRight)
            k_e if (HotKey::new(None, KbKey::ArrowRight)).matches(k_e) => {
                EditAction::Move(Movement::Right)
            }
            // Delete left word
            k_e if (HotKey::new(SysMods::Cmd, KbKey::Backspace)).matches(k_e) => {
                EditAction::JumpBackspace(Movement::LeftWord)
            }
            // Delete right word
            k_e if (HotKey::new(SysMods::Cmd, KbKey::Delete)).matches(k_e) => {
                EditAction::JumpDelete(Movement::RightWord)
            }
            // Backspace
            k_e if (HotKey::new(None, KbKey::Backspace)).matches(k_e) => EditAction::Backspace,
            // Delete
            k_e if (HotKey::new(None, KbKey::Delete)).matches(k_e) => EditAction::Delete,
            // Home
            k_e if (HotKey::new(None, KbKey::Home)).matches(k_e) => {
                EditAction::Move(Movement::PrecedingLineBreak)
            }
            // End
            k_e if (HotKey::new(None, KbKey::End)).matches(k_e) => {
                EditAction::Move(Movement::NextLineBreak)
            }
            // Actual typing
            k_e if key_event_is_printable(k_e) => {
                if let KbKey::Character(chars) = &k_e.key {
                    EditAction::Insert(chars.to_owned())
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        Some(action)
    }
}

/// Determine whether a keyboard event contains insertable text.
fn key_event_is_printable(event: &KeyEvent) -> bool {
    if let KbKey::Character(_) = &event.key {
        if event.mods.ctrl() || event.mods.meta() {
            return false;
        }
        // On mac, Alt functions more like AltGr.
        #[cfg(not(target_os = "macos"))]
        {
            if event.mods.alt() {
                return false;
            }
        }
        true
    } else {
        false
    }
}
