use super::Movement;
use crate::{HotKey, KeyCode, SysMods};
use druid_shell::KeyEvent;

// This following enumerations are heavily inspired by xi-editors enumerations found at
// https://github.com/xi-editor/xi-editor/blob/e2589974fc4050beb33af82481aa71b258358e48/rust/core-lib/src/edit_types.rs
// This is done with the goal of eventually being able to easily switch
// to a xi-based implementation of our Events.

/// Events that only modify view state
#[derive(Debug, PartialEq, Clone)]
pub enum ViewEvent {
    Move(Movement),
    ModifySelection(Movement),
    SelectAll,
    // Click(MouseAction),
    // Drag(MouseAction),
}

/// Events that modify the buffer
#[derive(Debug, PartialEq, Clone)]
pub enum BufferEvent {
    Delete, // { movement: Movement, kill: bool },
    Backspace,
    Insert(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum EventDomain {
    View(ViewEvent),
    Buffer(BufferEvent),
}

pub trait TextInput {
    /// Handle a key event and return an edit action to be executed
    /// for the key event
    fn handle_event(&self, event: &KeyEvent) -> Option<EventDomain>;
}

/// Handles key events and returns actions that are applicable to
/// single line textboxes
pub struct SingleLineTextInput {}

impl SingleLineTextInput {
    pub fn new() -> Self {
        Self {}
    }
}

impl TextInput for SingleLineTextInput {
    fn handle_event(&self, event: &KeyEvent) -> Option<EventDomain> {
        let action = match event {
            // Select all (Ctrl+A || Cmd+A)
            k_e if (HotKey::new(SysMods::Cmd, "a")).matches(k_e) => {
                EventDomain::View(ViewEvent::SelectAll)
            }
            // Jump left (Ctrl+ArrowLeft || Cmd+ArrowLeft)
            k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowLeft)).matches(k_e)
                || HotKey::new(None, KeyCode::Home).matches(k_e) =>
            {
                EventDomain::View(ViewEvent::Move(Movement::LeftOfLine))
            }
            // Jump right (Ctrl+ArrowRight || Cmd+ArrowRight)
            k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowRight)).matches(k_e)
                || HotKey::new(None, KeyCode::End).matches(k_e) =>
            {
                EventDomain::View(ViewEvent::Move(Movement::RightOfLine))
            }
            // Select left (Shift+ArrowLeft)
            k_e if (HotKey::new(SysMods::Shift, KeyCode::ArrowLeft)).matches(k_e) => {
                EventDomain::View(ViewEvent::ModifySelection(Movement::Left))
            }
            // Select right (Shift+ArrowRight)
            k_e if (HotKey::new(SysMods::Shift, KeyCode::ArrowRight)).matches(k_e) => {
                EventDomain::View(ViewEvent::ModifySelection(Movement::Right))
            }
            // Move left (ArrowLeft)
            k_e if (HotKey::new(None, KeyCode::ArrowLeft)).matches(k_e) => {
                EventDomain::View(ViewEvent::Move(Movement::Left))
            }
            // Move right (ArrowRight)
            k_e if (HotKey::new(None, KeyCode::ArrowRight)).matches(k_e) => {
                EventDomain::View(ViewEvent::Move(Movement::Right))
            }
            // Backspace
            k_e if (HotKey::new(None, KeyCode::Backspace)).matches(k_e) => {
                EventDomain::Buffer(BufferEvent::Backspace)
            }
            // Delete
            k_e if (HotKey::new(None, KeyCode::Delete)).matches(k_e) => {
                EventDomain::Buffer(BufferEvent::Delete)
            }
            // Actual typing
            k_e if k_e.key_code.is_printable() => {
                if let Some(chars) = k_e.text() {
                    EventDomain::Buffer(BufferEvent::Insert(chars.to_owned()))
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        Some(action)
    }
}
