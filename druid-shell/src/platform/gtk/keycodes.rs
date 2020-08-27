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

//! GTK code handling.

use gdk::keys::constants::*;

pub use super::super::shared::hardware_keycode_to_code;
use crate::keyboard_types::{Key, Location};

pub type RawKey = gdk::keys::Key;

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub fn raw_key_to_key(raw: RawKey) -> Option<Key> {
    Some(match raw {
        Escape => Key::Escape,
        BackSpace => Key::Backspace,
        Tab => Key::Tab,
        Return => Key::Enter,
        Control_L | Control_R => Key::Control,
        Alt_L | Alt_R => Key::Alt,
        Shift_L | Shift_R => Key::Shift,
        // TODO: investigate mapping. Map Meta_[LR]?
        Super_L | Super_R => Key::Meta,
        Caps_Lock => Key::CapsLock,
        F1 => Key::F1,
        F2 => Key::F2,
        F3 => Key::F3,
        F4 => Key::F4,
        F5 => Key::F5,
        F6 => Key::F6,
        F7 => Key::F7,
        F8 => Key::F8,
        F9 => Key::F9,
        F10 => Key::F10,
        F11 => Key::F11,
        F12 => Key::F12,

        Print => Key::PrintScreen,
        Scroll_Lock => Key::ScrollLock,
        // Pause/Break not audio.
        Pause => Key::Pause,

        Insert => Key::Insert,
        Delete => Key::Delete,
        Home => Key::Home,
        End => Key::End,
        Page_Up => Key::PageUp,
        Page_Down => Key::PageDown,
        Num_Lock => Key::NumLock,

        Up => Key::ArrowUp,
        Down => Key::ArrowDown,
        Left => Key::ArrowLeft,
        Right => Key::ArrowRight,
        Clear => Key::Clear,

        Menu => Key::ContextMenu,
        WakeUp => Key::WakeUp,
        Launch0 => Key::LaunchApplication1,
        Launch1 => Key::LaunchApplication2,
        ISO_Level3_Shift => Key::AltGraph,

        KP_Begin => Key::Clear,
        KP_Delete => Key::Delete,
        KP_Down => Key::ArrowDown,
        KP_End => Key::End,
        KP_Enter => Key::Enter,
        KP_F1 => Key::F1,
        KP_F2 => Key::F2,
        KP_F3 => Key::F3,
        KP_F4 => Key::F4,
        KP_Home => Key::Home,
        KP_Insert => Key::Insert,
        KP_Left => Key::ArrowLeft,
        KP_Page_Down => Key::PageDown,
        KP_Page_Up => Key::PageUp,
        KP_Right => Key::ArrowRight,
        // KP_Separator? What does it map to?
        KP_Tab => Key::Tab,
        KP_Up => Key::ArrowUp,
        // TODO: more mappings (media etc)
        _ => return None,
    })
}

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub fn raw_key_to_location(raw: RawKey) -> Location {
    match raw {
        Control_L | Shift_L | Alt_L | Super_L | Meta_L => Location::Left,
        Control_R | Shift_R | Alt_R | Super_R | Meta_R => Location::Right,
        KP_0 | KP_1 | KP_2 | KP_3 | KP_4 | KP_5 | KP_6 | KP_7 | KP_8 | KP_9 | KP_Add | KP_Begin
        | KP_Decimal | KP_Delete | KP_Divide | KP_Down | KP_End | KP_Enter | KP_Equal | KP_F1
        | KP_F2 | KP_F3 | KP_F4 | KP_Home | KP_Insert | KP_Left | KP_Multiply | KP_Page_Down
        | KP_Page_Up | KP_Right | KP_Separator | KP_Space | KP_Subtract | KP_Tab | KP_Up => {
            Location::Numpad
        }
        _ => Location::Standard,
    }
}

#[allow(non_upper_case_globals)]
pub fn key_to_raw_key(src: &Key) -> Option<RawKey> {
    Some(match src {
        Key::Escape => Escape,
        Key::Backspace => BackSpace,

        Key::Tab => Tab,
        Key::Enter => Return,

        // Give "left" variants
        Key::Control => Control_L,
        Key::Alt => Alt_L,
        Key::Shift => Shift_L,
        Key::Meta => Super_L,

        Key::CapsLock => Caps_Lock,
        Key::F1 => F1,
        Key::F2 => F2,
        Key::F3 => F3,
        Key::F4 => F4,
        Key::F5 => F5,
        Key::F6 => F6,
        Key::F7 => F7,
        Key::F8 => F8,
        Key::F9 => F9,
        Key::F10 => F10,
        Key::F11 => F11,
        Key::F12 => F12,

        Key::PrintScreen => Print,
        Key::ScrollLock => Scroll_Lock,
        // Pause/Break not audio.
        Key::Pause => Pause,

        Key::Insert => Insert,
        Key::Delete => Delete,
        Key::Home => Home,
        Key::End => End,
        Key::PageUp => Page_Up,
        Key::PageDown => Page_Down,

        Key::NumLock => Num_Lock,

        Key::ArrowUp => Up,
        Key::ArrowDown => Down,
        Key::ArrowLeft => Left,
        Key::ArrowRight => Right,

        Key::ContextMenu => Menu,
        Key::WakeUp => WakeUp,
        Key::LaunchApplication1 => Launch0,
        Key::LaunchApplication2 => Launch1,
        Key::AltGraph => ISO_Level3_Shift,
        // TODO: probably more
        _ => return None,
    })
}
