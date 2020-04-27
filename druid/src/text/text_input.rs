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

//! Map input to `EditAction`s

use super::Movement;
use crate::{HotKey, KeyCode, SysMods};
use druid_shell::{KeyEvent, KeyModifiers};

// This following enumerations are heavily inspired by xi-editors enumerations found at
// https://github.com/xi-editor/xi-editor/blob/e2589974fc4050beb33af82481aa71b258358e48/rust/core-lib/src/edit_types.rs
// This is done with the goal of eventually being able to easily switch
// to a xi-based implementation of our Events.

/// An enum that represents actions in a text buffer.
#[derive(Debug, PartialEq, Clone)]
pub enum EditAction {
    Move(Movement),
    ModifySelection(Movement),
    SelectAll,
    Click(MouseAction),
    Drag(MouseAction),
    Delete,
    Backspace,
    Insert(String),
    Paste(String),
}

/// Extra information related to mouse actions
#[derive(PartialEq, Debug, Clone)]
pub struct MouseAction {
    pub row: usize,
    pub column: usize,
    pub mods: KeyModifiers,
}

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
    pub fn new() -> Self {
        Self {}
    }
}

impl TextInput for BasicTextInput {
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        let action = match event {
            // Select all (Ctrl+A || Cmd+A)
            k_e if (HotKey::new(SysMods::Cmd, "a")).matches(k_e) => EditAction::SelectAll,
            // Jump left (Ctrl+ArrowLeft || Cmd+ArrowLeft)
            k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowLeft)).matches(k_e)
                || HotKey::new(None, KeyCode::Home).matches(k_e) =>
            {
                EditAction::Move(Movement::LeftOfLine)
            }
            // Jump right (Ctrl+ArrowRight || Cmd+ArrowRight)
            k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowRight)).matches(k_e)
                || HotKey::new(None, KeyCode::End).matches(k_e) =>
            {
                EditAction::Move(Movement::RightOfLine)
            }
            // Select left (Shift+ArrowLeft)
            k_e if (HotKey::new(SysMods::Shift, KeyCode::ArrowLeft)).matches(k_e) => {
                EditAction::ModifySelection(Movement::Left)
            }
            // Select right (Shift+ArrowRight)
            k_e if (HotKey::new(SysMods::Shift, KeyCode::ArrowRight)).matches(k_e) => {
                EditAction::ModifySelection(Movement::Right)
            }
            // Move left (ArrowLeft)
            k_e if (HotKey::new(None, KeyCode::ArrowLeft)).matches(k_e) => {
                EditAction::Move(Movement::Left)
            }
            // Move right (ArrowRight)
            k_e if (HotKey::new(None, KeyCode::ArrowRight)).matches(k_e) => {
                EditAction::Move(Movement::Right)
            }
            // Backspace
            k_e if (HotKey::new(None, KeyCode::Backspace)).matches(k_e) => EditAction::Backspace,
            // Delete
            k_e if (HotKey::new(None, KeyCode::Delete)).matches(k_e) => EditAction::Delete,
            // Actual typing
            k_e if k_e.key_code.is_printable() => {
                if let Some(chars) = k_e.text() {
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
