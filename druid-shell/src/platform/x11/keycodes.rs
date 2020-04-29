// Copyright 2020 The xi-editor Authors.
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

//! X11 keycode handling.

use gdk::enums::key::*;

use crate::keyboard::StrOrChar;
use crate::keycodes::KeyCode;

pub type RawKeyCode = u32;

impl From<RawKeyCode> for KeyCode {
    #[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
    fn from(raw: u32) -> KeyCode {
        match raw {
            9 => KeyCode::Escape,
            67 => KeyCode::F1,
            68 => KeyCode::F2,
            69 => KeyCode::F3,
            70 => KeyCode::F4,
            71 => KeyCode::F5,
            72 => KeyCode::F6,
            73 => KeyCode::F7,
            74 => KeyCode::F8,
            75 => KeyCode::F9,
            76 => KeyCode::F10,
            95 => KeyCode::F11,
            96 => KeyCode::F12,

            107 => KeyCode::PrintScreen,
            118 => KeyCode::Insert,
            119 => KeyCode::Delete,
            49 => KeyCode::Quote,

            10 => KeyCode::Numpad1,
            11 => KeyCode::Numpad2,
            12 => KeyCode::Numpad3,
            13 => KeyCode::Numpad4,
            14 => KeyCode::Numpad5,
            15 => KeyCode::Numpad6,
            16 => KeyCode::Numpad7,
            17 => KeyCode::Numpad8,
            18 => KeyCode::Numpad9,
            19 => KeyCode::Numpad0,

            20 => KeyCode::LeftBracket,
            21 => KeyCode::RightBracket,
            22 => KeyCode::Backspace,

            23 => KeyCode::Tab,

            24 => KeyCode::KeyQ,
            25 => KeyCode::KeyW,
            26 => KeyCode::KeyE,
            27 => KeyCode::KeyR,
            28 => KeyCode::KeyT,
            29 => KeyCode::KeyY,
            30 => KeyCode::KeyU,
            31 => KeyCode::KeyI,
            32 => KeyCode::KeyO,
            33 => KeyCode::KeyP,

            34 => KeyCode::Slash,
            35 => KeyCode::Equals,
            36 => KeyCode::Return,

            66 => KeyCode::CapsLock,

            38 => KeyCode::KeyA,
            39 => KeyCode::KeyS,
            40 => KeyCode::KeyD,
            41 => KeyCode::KeyF,
            42 => KeyCode::KeyG,
            43 => KeyCode::KeyH,
            44 => KeyCode::KeyJ,
            45 => KeyCode::KeyK,
            46 => KeyCode::KeyL,

            47 => KeyCode::Semicolon,
            48 => KeyCode::Comma,
            // 49 => KeyCode::Hash,
            50 => KeyCode::LeftShift,
            94 => KeyCode::Backslash,

            52 => KeyCode::KeyZ,
            53 => KeyCode::KeyX,
            54 => KeyCode::KeyC,
            55 => KeyCode::KeyV,
            56 => KeyCode::KeyB,
            57 => KeyCode::KeyN,
            58 => KeyCode::KeyM,

            59 => KeyCode::Comma,
            60 => KeyCode::Period,
            61 => KeyCode::NumpadMultiply,
            64 => KeyCode::RightShift,

            37 => KeyCode::LeftControl,
            133 => KeyCode::LeftMeta,
            // TODO(x11/keycodes): case 64 is duplicated; which one is correct?
            //     What should the other be?
            // 64 => KeyCode::LeftAlt,
            65 => KeyCode::Space,
            108 => KeyCode::RightAlt,
            105 => KeyCode::RightControl,

            111 => KeyCode::ArrowUp,
            113 => KeyCode::ArrowLeft,
            114 => KeyCode::ArrowRight,
            116 => KeyCode::ArrowDown,

            _ => {
                log::warn!("Warning: unknown keyval {}", raw);
                KeyCode::Unknown(raw)
            }
        }
    }
}

impl Into<StrOrChar<'static>> for KeyCode {
    #[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
    fn into(self) -> StrOrChar<'static> {
        match self {
            KeyCode::Numpad1 => StrOrChar::Char('1'),
            KeyCode::Numpad2 => StrOrChar::Char('2'),
            KeyCode::Numpad3 => StrOrChar::Char('3'),
            KeyCode::Numpad4 => StrOrChar::Char('4'),
            KeyCode::Numpad5 => StrOrChar::Char('5'),
            KeyCode::Numpad6 => StrOrChar::Char('6'),
            KeyCode::Numpad7 => StrOrChar::Char('7'),
            KeyCode::Numpad8 => StrOrChar::Char('8'),
            KeyCode::Numpad9 => StrOrChar::Char('9'),
            KeyCode::Numpad0 => StrOrChar::Char('0'),

            KeyCode::LeftBracket => StrOrChar::Char('['),
            KeyCode::RightBracket => StrOrChar::Char(']'),
            // KeyCode::Backspace => StrOrChar::Char('0'),
            // KeyCode::Tab => StrOrChar::Char('0'),
            KeyCode::KeyQ => StrOrChar::Char('q'),
            KeyCode::KeyW => StrOrChar::Char('w'),
            KeyCode::KeyE => StrOrChar::Char('e'),
            KeyCode::KeyR => StrOrChar::Char('r'),
            KeyCode::KeyT => StrOrChar::Char('t'),
            KeyCode::KeyY => StrOrChar::Char('y'),
            KeyCode::KeyU => StrOrChar::Char('u'),
            KeyCode::KeyI => StrOrChar::Char('i'),
            KeyCode::KeyO => StrOrChar::Char('o'),
            KeyCode::KeyP => StrOrChar::Char('p'),
            KeyCode::Slash => StrOrChar::Char('/'),
            KeyCode::Equals => StrOrChar::Char('='),
            // KeyCode::Return => StrOrChar::Char('0'),
            // KeyCode::CapsLock => StrOrChar::Char('0'),
            KeyCode::KeyA => StrOrChar::Char('a'),
            KeyCode::KeyS => StrOrChar::Char('s'),
            KeyCode::KeyD => StrOrChar::Char('d'),
            KeyCode::KeyF => StrOrChar::Char('f'),
            KeyCode::KeyG => StrOrChar::Char('g'),
            KeyCode::KeyH => StrOrChar::Char('h'),
            KeyCode::KeyJ => StrOrChar::Char('j'),
            KeyCode::KeyK => StrOrChar::Char('k'),
            KeyCode::KeyL => StrOrChar::Char('l'),
            KeyCode::Semicolon => StrOrChar::Char(';'),
            // KeyCode::Hash => StrOrChar::Char('#'),
            // KeyCode::LeftShift => StrOrChar::Char('0'),
            KeyCode::Backslash => StrOrChar::Char('/'),

            KeyCode::KeyZ => StrOrChar::Char('z'),
            KeyCode::KeyX => StrOrChar::Char('x'),
            KeyCode::KeyC => StrOrChar::Char('c'),
            KeyCode::KeyV => StrOrChar::Char('v'),
            KeyCode::KeyB => StrOrChar::Char('b'),
            KeyCode::KeyN => StrOrChar::Char('n'),
            KeyCode::KeyM => StrOrChar::Char('m'),
            KeyCode::Comma => StrOrChar::Char(','),
            KeyCode::Period => StrOrChar::Char('.'),
            KeyCode::NumpadMultiply => StrOrChar::Char('*'),
            // KeyCode::RightShift => StrOrChar::Char('0'),
            // KeyCode::LeftControl => StrOrChar::Char('0'),
            // KeyCode::LeftMeta => StrOrChar::Char('0'),
            // KeyCode::LeftAlt => StrOrChar::Char('0'),
            KeyCode::Space => StrOrChar::Char(' '),
            // KeyCode::RightAlt => StrOrChar::Char('0'),
            // KeyCode::RightControl => StrOrChar::Char('0'),
            /*
               KeyCode::ArrowUp
               KeyCode::ArrowLeft
               KeyCode::ArrowRight
               KeyCode::ArrowDown
            */
            _ => {
                log::warn!("Warning: unknown keycode str representation {:?}", self);
                StrOrChar::Char('?')
            }
        }
    }
}

impl From<KeyCode> for u32 {
    #[allow(non_upper_case_globals)]
    fn from(src: KeyCode) -> u32 {
        match src {
            KeyCode::Escape => Escape,
            KeyCode::Backtick => grave,
            KeyCode::Key0 => _0,
            KeyCode::Key1 => _1,
            KeyCode::Key2 => _2,
            KeyCode::Key3 => _3,
            KeyCode::Key4 => _4,
            KeyCode::Key5 => _5,
            KeyCode::Key6 => _6,
            KeyCode::Key7 => _7,
            KeyCode::Key8 => _8,
            KeyCode::Key9 => _9,
            KeyCode::Minus => minus,
            KeyCode::Equals => equal,
            KeyCode::Backspace => BackSpace,

            KeyCode::Tab => Tab,
            KeyCode::KeyQ => q | Q,
            KeyCode::KeyW => w | W,
            KeyCode::KeyE => e | E,
            KeyCode::KeyR => r | R,
            KeyCode::KeyT => t | T,
            KeyCode::KeyY => y | Y,
            KeyCode::KeyU => u | U,
            KeyCode::KeyI => i | I,
            KeyCode::KeyO => o | O,
            KeyCode::KeyP => p | P,
            KeyCode::LeftBracket => bracketleft,
            KeyCode::RightBracket => bracketright,
            KeyCode::Return => Return,

            KeyCode::KeyA => a | A,
            KeyCode::KeyS => s | S,
            KeyCode::KeyD => d | D,
            KeyCode::KeyF => f | F,
            KeyCode::KeyG => g | G,
            KeyCode::KeyH => h | H,
            KeyCode::KeyJ => j | J,
            KeyCode::KeyK => k | K,
            KeyCode::KeyL => l | L,
            KeyCode::Semicolon => semicolon,
            KeyCode::Quote => quoteright,
            KeyCode::Backslash => backslash,

            KeyCode::KeyZ => z | Z,
            KeyCode::KeyX => x | X,
            KeyCode::KeyC => c | C,
            KeyCode::KeyV => v | V,
            KeyCode::KeyB => b | B,
            KeyCode::KeyN => n | N,
            KeyCode::KeyM => m | M,
            KeyCode::Comma => comma,
            KeyCode::Period => period,
            KeyCode::Slash => slash,

            KeyCode::LeftControl => Control_L,
            KeyCode::RightControl => Control_R,
            KeyCode::LeftAlt => Alt_L,
            KeyCode::RightAlt => Alt_R,
            KeyCode::LeftShift => Shift_L,
            KeyCode::RightShift => Shift_R,
            KeyCode::LeftMeta => Super_L,
            KeyCode::RightMeta => Super_R,

            KeyCode::Space => space,
            KeyCode::CapsLock => Caps_Lock,
            KeyCode::F1 => F1,
            KeyCode::F2 => F2,
            KeyCode::F3 => F3,
            KeyCode::F4 => F4,
            KeyCode::F5 => F5,
            KeyCode::F6 => F6,
            KeyCode::F7 => F7,
            KeyCode::F8 => F8,
            KeyCode::F9 => F9,
            KeyCode::F10 => F10,
            KeyCode::F11 => F11,
            KeyCode::F12 => F12,

            KeyCode::PrintScreen => Print,
            KeyCode::ScrollLock => Scroll_Lock,
            // Pause/Break not audio.
            KeyCode::Pause => Pause,

            KeyCode::Insert => Insert,
            KeyCode::Delete => Delete,
            KeyCode::Home => Home,
            KeyCode::End => End,
            KeyCode::PageUp => Page_Up,
            KeyCode::PageDown => Page_Down,

            KeyCode::Numpad0 => KP_0,
            KeyCode::Numpad1 => KP_1,
            KeyCode::Numpad2 => KP_2,
            KeyCode::Numpad3 => KP_3,
            KeyCode::Numpad4 => KP_4,
            KeyCode::Numpad5 => KP_5,
            KeyCode::Numpad6 => KP_6,
            KeyCode::Numpad7 => KP_7,
            KeyCode::Numpad8 => KP_8,
            KeyCode::Numpad9 => KP_9,

            KeyCode::NumpadEquals => KP_Equal,
            KeyCode::NumpadSubtract => KP_Subtract,
            KeyCode::NumpadAdd => KP_Add,
            KeyCode::NumpadDecimal => KP_Decimal,
            KeyCode::NumpadMultiply => KP_Multiply,
            KeyCode::NumpadDivide => KP_Divide,
            KeyCode::NumLock => Num_Lock,
            KeyCode::NumpadEnter => KP_Enter,

            KeyCode::ArrowUp => Up,
            KeyCode::ArrowDown => Down,
            KeyCode::ArrowLeft => Left,
            KeyCode::ArrowRight => Right,

            // We don't know what this keycode is, so just return it directly.
            KeyCode::Unknown(unknown_keycode) => unknown_keycode,
        }
    }
}
