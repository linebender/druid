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

//! GTK code handling.

use gdk::enums::key::*;

use crate::keyboard_types::{Code, Key, Location};

pub type RawKey = gdk::enums::key::Key;

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

/// Map hardware keycode to code.
///
/// In theory, the hardware keycode is device dependent, but in
/// practice it's probably pretty reliable.
///
/// The logic is based on NativeKeyToDOMCodeName.h
pub fn hardware_keycode_to_code(hw_keycode: u16) -> Code {
    match hw_keycode {
        0x0009 => Code::Escape,
        0x000A => Code::Digit1,
        0x000B => Code::Digit2,
        0x000C => Code::Digit3,
        0x000D => Code::Digit4,
        0x000E => Code::Digit5,
        0x000F => Code::Digit6,
        0x0010 => Code::Digit7,
        0x0011 => Code::Digit8,
        0x0012 => Code::Digit9,
        0x0013 => Code::Digit0,
        0x0014 => Code::Minus,
        0x0015 => Code::Equal,
        0x0016 => Code::Backspace,
        0x0017 => Code::Tab,
        0x0018 => Code::KeyQ,
        0x0019 => Code::KeyW,
        0x001A => Code::KeyE,
        0x001B => Code::KeyR,
        0x001C => Code::KeyT,
        0x001D => Code::KeyY,
        0x001E => Code::KeyU,
        0x001F => Code::KeyI,
        0x0020 => Code::KeyO,
        0x0021 => Code::KeyP,
        0x0022 => Code::BracketLeft,
        0x0023 => Code::BracketRight,
        0x0024 => Code::Enter,
        0x0025 => Code::ControlLeft,
        0x0026 => Code::KeyA,
        0x0027 => Code::KeyS,
        0x0028 => Code::KeyD,
        0x0029 => Code::KeyF,
        0x002A => Code::KeyG,
        0x002B => Code::KeyH,
        0x002C => Code::KeyJ,
        0x002D => Code::KeyK,
        0x002E => Code::KeyL,
        0x002F => Code::Semicolon,
        0x0030 => Code::Quote,
        0x0031 => Code::Backquote,
        0x0032 => Code::ShiftLeft,
        0x0033 => Code::Backslash,
        0x0034 => Code::KeyZ,
        0x0035 => Code::KeyX,
        0x0036 => Code::KeyC,
        0x0037 => Code::KeyV,
        0x0038 => Code::KeyB,
        0x0039 => Code::KeyN,
        0x003A => Code::KeyM,
        0x003B => Code::Comma,
        0x003C => Code::Period,
        0x003D => Code::Slash,
        0x003E => Code::ShiftRight,
        0x003F => Code::NumpadMultiply,
        0x0040 => Code::AltLeft,
        0x0041 => Code::Space,
        0x0042 => Code::CapsLock,
        0x0043 => Code::F1,
        0x0044 => Code::F2,
        0x0045 => Code::F3,
        0x0046 => Code::F4,
        0x0047 => Code::F5,
        0x0048 => Code::F6,
        0x0049 => Code::F7,
        0x004A => Code::F8,
        0x004B => Code::F9,
        0x004C => Code::F10,
        0x004D => Code::NumLock,
        0x004E => Code::ScrollLock,
        0x004F => Code::Numpad7,
        0x0050 => Code::Numpad8,
        0x0051 => Code::Numpad9,
        0x0052 => Code::NumpadSubtract,
        0x0053 => Code::Numpad4,
        0x0054 => Code::Numpad5,
        0x0055 => Code::Numpad6,
        0x0056 => Code::NumpadAdd,
        0x0057 => Code::Numpad1,
        0x0058 => Code::Numpad2,
        0x0059 => Code::Numpad3,
        0x005A => Code::Numpad0,
        0x005B => Code::NumpadDecimal,
        0x005E => Code::IntlBackslash,
        0x005F => Code::F11,
        0x0060 => Code::F12,
        0x0061 => Code::IntlRo,
        0x0064 => Code::Convert,
        0x0065 => Code::KanaMode,
        0x0066 => Code::NonConvert,
        0x0068 => Code::NumpadEnter,
        0x0069 => Code::ControlRight,
        0x006A => Code::NumpadDivide,
        0x006B => Code::PrintScreen,
        0x006C => Code::AltRight,
        0x006E => Code::Home,
        0x006F => Code::ArrowUp,
        0x0070 => Code::PageUp,
        0x0071 => Code::ArrowLeft,
        0x0072 => Code::ArrowRight,
        0x0073 => Code::End,
        0x0074 => Code::ArrowDown,
        0x0075 => Code::PageDown,
        0x0076 => Code::Insert,
        0x0077 => Code::Delete,
        0x0079 => Code::AudioVolumeMute,
        0x007A => Code::AudioVolumeDown,
        0x007B => Code::AudioVolumeUp,
        0x007D => Code::NumpadEqual,
        0x007F => Code::Pause,
        0x0081 => Code::NumpadComma,
        0x0082 => Code::Lang1,
        0x0083 => Code::Lang2,
        0x0084 => Code::IntlYen,
        0x0085 => Code::MetaLeft,
        0x0086 => Code::MetaRight,
        0x0087 => Code::ContextMenu,
        0x0088 => Code::BrowserStop,
        0x0089 => Code::Again,
        0x008A => Code::Props,
        0x008B => Code::Undo,
        0x008C => Code::Select,
        0x008D => Code::Copy,
        0x008E => Code::Open,
        0x008F => Code::Paste,
        0x0090 => Code::Find,
        0x0091 => Code::Cut,
        0x0092 => Code::Help,
        0x0094 => Code::LaunchApp2,
        0x0097 => Code::WakeUp,
        0x0098 => Code::LaunchApp1,
        // key to right of volume controls on T430s produces 0x9C
        // but no documentation of what it should map to :/
        0x00A3 => Code::LaunchMail,
        0x00A4 => Code::BrowserFavorites,
        0x00A6 => Code::BrowserBack,
        0x00A7 => Code::BrowserForward,
        0x00A9 => Code::Eject,
        0x00AB => Code::MediaTrackNext,
        0x00AC => Code::MediaPlayPause,
        0x00AD => Code::MediaTrackPrevious,
        0x00AE => Code::MediaStop,
        0x00B3 => Code::MediaSelect,
        0x00B4 => Code::BrowserHome,
        0x00B5 => Code::BrowserRefresh,
        0x00E1 => Code::BrowserSearch,
        _ => Code::Unidentified,
    }
}

// TODO: this feels wrong to me but might be reasonable
// for menus. We should probably be mapping Key and not
// Code.
#[allow(non_upper_case_globals)]
pub fn code_to_raw_key(src: Code) -> u32 {
    match src {
        Code::Escape => Escape,
        Code::Backquote => grave,
        Code::Digit0 => _0,
        Code::Digit1 => _1,
        Code::Digit2 => _2,
        Code::Digit3 => _3,
        Code::Digit4 => _4,
        Code::Digit5 => _5,
        Code::Digit6 => _6,
        Code::Digit7 => _7,
        Code::Digit8 => _8,
        Code::Digit9 => _9,
        Code::Minus => minus,
        Code::Equal => equal,
        Code::Backspace => BackSpace,

        Code::Tab => Tab,
        Code::KeyQ => q | Q,
        Code::KeyW => w | W,
        Code::KeyE => e | E,
        Code::KeyR => r | R,
        Code::KeyT => t | T,
        Code::KeyY => y | Y,
        Code::KeyU => u | U,
        Code::KeyI => i | I,
        Code::KeyO => o | O,
        Code::KeyP => p | P,
        Code::BracketLeft => bracketleft,
        Code::BracketRight => bracketright,
        Code::Enter => Return,

        Code::KeyA => a | A,
        Code::KeyS => s | S,
        Code::KeyD => d | D,
        Code::KeyF => f | F,
        Code::KeyG => g | G,
        Code::KeyH => h | H,
        Code::KeyJ => j | J,
        Code::KeyK => k | K,
        Code::KeyL => l | L,
        Code::Semicolon => semicolon,
        Code::Quote => quoteright,
        Code::Backslash => backslash,

        Code::KeyZ => z | Z,
        Code::KeyX => x | X,
        Code::KeyC => c | C,
        Code::KeyV => v | V,
        Code::KeyB => b | B,
        Code::KeyN => n | N,
        Code::KeyM => m | M,
        Code::Comma => comma,
        Code::Period => period,
        Code::Slash => slash,

        Code::ControlLeft => Control_L,
        Code::ControlRight => Control_R,
        Code::AltLeft => Alt_L,
        Code::AltRight => Alt_R,
        Code::ShiftLeft => Shift_L,
        Code::ShiftRight => Shift_R,
        Code::MetaLeft => Super_L,
        Code::MetaRight => Super_R,

        Code::Space => space,
        Code::CapsLock => Caps_Lock,
        Code::F1 => F1,
        Code::F2 => F2,
        Code::F3 => F3,
        Code::F4 => F4,
        Code::F5 => F5,
        Code::F6 => F6,
        Code::F7 => F7,
        Code::F8 => F8,
        Code::F9 => F9,
        Code::F10 => F10,
        Code::F11 => F11,
        Code::F12 => F12,

        Code::PrintScreen => Print,
        Code::ScrollLock => Scroll_Lock,
        // Pause/Break not audio.
        Code::Pause => Pause,

        Code::Insert => Insert,
        Code::Delete => Delete,
        Code::Home => Home,
        Code::End => End,
        Code::PageUp => Page_Up,
        Code::PageDown => Page_Down,

        Code::Numpad0 => KP_0,
        Code::Numpad1 => KP_1,
        Code::Numpad2 => KP_2,
        Code::Numpad3 => KP_3,
        Code::Numpad4 => KP_4,
        Code::Numpad5 => KP_5,
        Code::Numpad6 => KP_6,
        Code::Numpad7 => KP_7,
        Code::Numpad8 => KP_8,
        Code::Numpad9 => KP_9,

        Code::NumpadEqual => KP_Equal,
        Code::NumpadSubtract => KP_Subtract,
        Code::NumpadAdd => KP_Add,
        Code::NumpadDecimal => KP_Decimal,
        Code::NumpadMultiply => KP_Multiply,
        Code::NumpadDivide => KP_Divide,
        Code::NumLock => Num_Lock,
        Code::NumpadEnter => KP_Enter,

        Code::ArrowUp => Up,
        Code::ArrowDown => Down,
        Code::ArrowLeft => Left,
        Code::ArrowRight => Right,
        _ => unreachable!("Unreachable: converting unknown Code {:?} to a keyval", src),
    }
}
