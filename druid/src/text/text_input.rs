use crate::{HotKey, KeyCode, SysMods};
use druid_shell::KeyEvent;

// This enumeration is heavily inspired by xi-editors edit notification
// https://github.com/xi-editor/xi-editor/blob/066523b2a57f719cd93cc9ac0dda7687194badd4/rust/core-lib/src/rpc.rs#L374
// This is done with the goal of eventually being able to easily switch
// to a xi-based implementation of our EditActions.
#[derive(Clone, Debug)]
pub enum EditAction {
    Insert { chars: String },
    DeleteForward,
    DeleteBackward,

    MoveLeft,
    MoveLeftAndModifySelection,
    MoveRight,
    MoveRightAndModifySelection,

    MoveToLeftEndOfLine,
    MoveToRightEndOfLine,

    SelectAll,
}

pub trait TextInput {
    /// Handle a key event and return an edit action to be executed
    /// for the key event
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction>;
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
    fn handle_event(&self, event: &KeyEvent) -> Option<EditAction> {
        let action = match event {
            // Select all (Ctrl+A || Cmd+A)
            k_e if (HotKey::new(SysMods::Cmd, "a")).matches(k_e) => EditAction::SelectAll,
            // Jump left (Ctrl+ArrowLeft || Cmd+ArrowLeft)
            k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowLeft)).matches(k_e)
                || HotKey::new(None, KeyCode::Home).matches(k_e) =>
            {
                EditAction::MoveToLeftEndOfLine
            }
            // Jump right (Ctrl+ArrowRight || Cmd+ArrowRight)
            k_e if (HotKey::new(SysMods::Cmd, KeyCode::ArrowRight)).matches(k_e)
                || HotKey::new(None, KeyCode::End).matches(k_e) =>
            {
                EditAction::MoveToRightEndOfLine
            }
            // Select left (Shift+ArrowLeft)
            k_e if (HotKey::new(SysMods::Shift, KeyCode::ArrowLeft)).matches(k_e) => {
                EditAction::MoveLeftAndModifySelection
            }
            // Select right (Shift+ArrowRight)
            k_e if (HotKey::new(SysMods::Shift, KeyCode::ArrowRight)).matches(k_e) => {
                EditAction::MoveRightAndModifySelection
            }
            // Move left (ArrowLeft)
            k_e if (HotKey::new(None, KeyCode::ArrowLeft)).matches(k_e) => EditAction::MoveLeft,
            // Move right (ArrowRight)
            k_e if (HotKey::new(None, KeyCode::ArrowRight)).matches(k_e) => EditAction::MoveRight,
            // Backspace
            k_e if (HotKey::new(None, KeyCode::Backspace)).matches(k_e) => {
                EditAction::DeleteBackward
            }
            // Delete
            k_e if (HotKey::new(None, KeyCode::Delete)).matches(k_e) => EditAction::DeleteForward,
            // Actual typing
            k_e if k_e.key_code.is_printable() => {
                if let Some(chars) = k_e.text() {
                    EditAction::Insert {
                        chars: chars.to_owned(),
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        Some(action)
    }
}
