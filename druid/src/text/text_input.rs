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
use crate::{HotKey, KbKey, KeyEvent, Modifiers, RawMods, SysMods};

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
#[derive(Default, Debug, Clone)]
pub struct BasicTextInput;

/// Common to all platforms.
#[derive(Default, Debug, Clone)]
struct SharedTextInput;

impl BasicTextInput {
    /// Create a new `BasicTextInput`.
    pub fn new() -> Self {
        Self
    }
}

impl TextInput for BasicTextInput {
    #[cfg(target_os = "macos")]
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        MacOSBasicTextInput
            .handle_event(event)
            .or_else(|| SharedTextInput.handle_event(event))
    }

    #[cfg(not(target_os = "macos"))]
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        GenericNonMacOSTextInput
            .handle_event(event)
            .or_else(|| SharedTextInput.handle_event(event))
    }
}

impl TextInput for SharedTextInput {
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        let action = match event {
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
            // Move left (ArrowLeft)
            k_e if (HotKey::new(None, KbKey::ArrowLeft)).matches(k_e) => {
                EditAction::Move(Movement::Left)
            }
            // Move right (ArrowRight)
            k_e if (HotKey::new(None, KbKey::ArrowRight)).matches(k_e) => {
                EditAction::Move(Movement::Right)
            }
            k_e if (HotKey::new(None, KbKey::ArrowUp)).matches(k_e) => {
                EditAction::Move(Movement::Up)
            }
            k_e if (HotKey::new(None, KbKey::ArrowDown)).matches(k_e) => {
                EditAction::Move(Movement::Down)
            }
            k_e if (HotKey::new(SysMods::Shift, KbKey::ArrowUp)).matches(k_e) => {
                EditAction::ModifySelection(Movement::Up)
            }
            k_e if (HotKey::new(SysMods::Shift, KbKey::ArrowDown)).matches(k_e) => {
                EditAction::ModifySelection(Movement::Down)
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
            k_e => match string_from_key(k_e) {
                Some(txt) => EditAction::Insert(txt),
                None => return None,
            },
        };

        Some(action)
    }
}

/// A handler for "not macOS"; this may need to get split up at some point.
#[cfg(not(target_os = "macos"))]
struct GenericNonMacOSTextInput;

/// A handler for macos-specific actions.
#[cfg(target_os = "macos")]
struct MacOSBasicTextInput;

#[cfg(not(target_os = "macos"))]
impl TextInput for GenericNonMacOSTextInput {
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        match event {
            // Delete left word
            k_e if (HotKey::new(RawMods::Ctrl, KbKey::Backspace)).matches(k_e) => {
                Some(EditAction::JumpBackspace(Movement::LeftWord))
            }
            // Delete right word
            k_e if (HotKey::new(RawMods::Ctrl, KbKey::Delete)).matches(k_e) => {
                Some(EditAction::JumpDelete(Movement::RightWord))
            }
            k_e if (HotKey::new(RawMods::Ctrl, KbKey::ArrowLeft)).matches(k_e) => {
                Some(EditAction::Move(Movement::LeftWord))
            }
            k_e if (HotKey::new(RawMods::Ctrl, KbKey::ArrowRight)).matches(k_e) => {
                Some(EditAction::Move(Movement::RightWord))
            }
            // Select left word
            k_e if (HotKey::new(RawMods::CtrlShift, KbKey::ArrowLeft)).matches(k_e) => {
                Some(EditAction::ModifySelection(Movement::LeftWord))
            }
            // Select right word
            k_e if (HotKey::new(RawMods::CtrlShift, KbKey::ArrowRight)).matches(k_e) => {
                Some(EditAction::ModifySelection(Movement::RightWord))
            }
            _ => None,
        }
    }
}

#[cfg(target_os = "macos")]
impl TextInput for MacOSBasicTextInput {
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        match event {
            k_e if HotKey::new(RawMods::AltCtrl, KbKey::Backspace).matches(k_e)
                || HotKey::new(RawMods::Alt, KbKey::Backspace).matches(k_e) =>
            {
                Some(EditAction::JumpBackspace(Movement::LeftWord))
            }
            k_e if HotKey::new(RawMods::AltCtrl, KbKey::Delete).matches(k_e) => {
                Some(EditAction::JumpBackspace(Movement::RightWord))
            }
            k_e if HotKey::new(RawMods::Meta, KbKey::ArrowLeft).matches(k_e) => {
                Some(EditAction::Move(Movement::PrecedingLineBreak))
            }
            k_e if HotKey::new(RawMods::MetaShift, KbKey::ArrowLeft).matches(k_e) => {
                Some(EditAction::ModifySelection(Movement::PrecedingLineBreak))
            }
            k_e if HotKey::new(RawMods::Alt, KbKey::ArrowLeft).matches(k_e) => {
                Some(EditAction::Move(Movement::LeftWord))
            }
            k_e if HotKey::new(RawMods::AltShift, KbKey::ArrowLeft).matches(k_e) => {
                Some(EditAction::ModifySelection(Movement::LeftWord))
            }
            k_e if HotKey::new(RawMods::Meta, KbKey::ArrowRight).matches(k_e) => {
                Some(EditAction::Move(Movement::NextLineBreak))
            }
            k_e if HotKey::new(RawMods::MetaShift, KbKey::ArrowRight).matches(k_e) => {
                Some(EditAction::ModifySelection(Movement::NextLineBreak))
            }
            k_e if HotKey::new(RawMods::Alt, KbKey::ArrowRight).matches(k_e) => {
                Some(EditAction::Move(Movement::RightWord))
            }
            k_e if HotKey::new(RawMods::AltShift, KbKey::ArrowRight).matches(k_e) => {
                Some(EditAction::ModifySelection(Movement::RightWord))
            }

            //readline things: these could probably be shared?
            k_e if HotKey::new(RawMods::Ctrl, KbKey::Character("a".into())).matches(k_e) => {
                Some(EditAction::Move(Movement::PrecedingLineBreak))
            }
            k_e if HotKey::new(RawMods::Ctrl, KbKey::Character("e".into())).matches(k_e) => {
                Some(EditAction::Move(Movement::NextLineBreak))
            }
            k_e if HotKey::new(RawMods::Ctrl, KbKey::Character("d".into())).matches(k_e) => {
                Some(EditAction::Delete)
            }
            k_e if HotKey::new(RawMods::Ctrl, KbKey::Character("b".into())).matches(k_e) => {
                Some(EditAction::Delete)
            }
            _ => None,
        }
    }
}

/// Determine whether a keyboard event contains insertable text.
fn string_from_key(event: &KeyEvent) -> Option<String> {
    match &event.key {
        KbKey::Character(_) if event.mods.ctrl() || event.mods.meta() => None,
        #[cfg(not(target_os = "macos"))]
        KbKey::Character(_) if event.mods.alt() => None,
        KbKey::Character(chars) => Some(chars.to_owned()),
        KbKey::Enter => Some("\n".into()),
        KbKey::Tab => Some("\t".into()),
        _ => None,
    }
}
