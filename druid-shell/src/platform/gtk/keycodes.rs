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

//! GTK keycode handling.

use gdk::enums::key::*;

use crate::keycodes::KeyCode;

pub type RawKeyCode = u32;

impl From<u32> for KeyCode {
    #[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
    fn from(raw: u32) -> KeyCode {
        match raw {
            Escape => KeyCode::Escape,
            grave => KeyCode::Backtick,
            _0 => KeyCode::Key0,
            _1 => KeyCode::Key1,
            _2 => KeyCode::Key2,
            _3 => KeyCode::Key3,
            _4 => KeyCode::Key4,
            _5 => KeyCode::Key5,
            _6 => KeyCode::Key6,
            _7 => KeyCode::Key7,
            _8 => KeyCode::Key8,
            _9 => KeyCode::Key9,
            minus => KeyCode::Minus,
            equal => KeyCode::Equals,
            BackSpace => KeyCode::Backspace,

            Tab => KeyCode::Tab,
            q | Q => KeyCode::KeyQ,
            w | W => KeyCode::KeyW,
            e | E => KeyCode::KeyE,
            r | R => KeyCode::KeyR,
            t | T => KeyCode::KeyT,
            y | Y => KeyCode::KeyY,
            u | U => KeyCode::KeyU,
            i | I => KeyCode::KeyI,
            o | O => KeyCode::KeyO,
            p | P => KeyCode::KeyP,
            bracketleft => KeyCode::LeftBracket,
            bracketright => KeyCode::RightBracket,
            Return => KeyCode::Return,

            a | A => KeyCode::KeyA,
            s | S => KeyCode::KeyS,
            d | D => KeyCode::KeyD,
            f | F => KeyCode::KeyF,
            g | G => KeyCode::KeyG,
            h | H => KeyCode::KeyH,
            j | J => KeyCode::KeyJ,
            k | K => KeyCode::KeyK,
            l | L => KeyCode::KeyL,
            semicolon => KeyCode::Semicolon,
            quoteright => KeyCode::Quote,
            backslash => KeyCode::Backslash,

            z | Z => KeyCode::KeyZ,
            x | X => KeyCode::KeyX,
            c | C => KeyCode::KeyC,
            v | V => KeyCode::KeyV,
            b | B => KeyCode::KeyB,
            n | N => KeyCode::KeyN,
            m | M => KeyCode::KeyM,
            comma => KeyCode::Comma,
            period => KeyCode::Period,
            slash => KeyCode::Slash,

            Control_L => KeyCode::LeftControl,
            Control_R => KeyCode::RightControl,
            Alt_L => KeyCode::LeftAlt,
            Alt_R => KeyCode::RightAlt,
            Shift_L => KeyCode::LeftShift,
            Shift_R => KeyCode::RightShift,
            Super_L => KeyCode::LeftMeta,
            Super_R => KeyCode::RightMeta,

            space => KeyCode::Space,
            Caps_Lock => KeyCode::CapsLock,
            F1 => KeyCode::F1,
            F2 => KeyCode::F2,
            F3 => KeyCode::F3,
            F4 => KeyCode::F4,
            F5 => KeyCode::F5,
            F6 => KeyCode::F6,
            F7 => KeyCode::F7,
            F8 => KeyCode::F8,
            F9 => KeyCode::F9,
            F10 => KeyCode::F10,
            F11 => KeyCode::F11,
            F12 => KeyCode::F12,

            Print => KeyCode::PrintScreen,
            Scroll_Lock => KeyCode::ScrollLock,
            // Pause/Break not audio.
            Pause => KeyCode::Pause,

            Insert => KeyCode::Insert,
            Delete => KeyCode::Delete,
            Home => KeyCode::Home,
            End => KeyCode::End,
            Page_Up => KeyCode::PageUp,
            Page_Down => KeyCode::PageDown,

            KP_0 => KeyCode::Numpad0,
            KP_1 => KeyCode::Numpad1,
            KP_2 => KeyCode::Numpad2,
            KP_3 => KeyCode::Numpad3,
            KP_4 => KeyCode::Numpad4,
            KP_5 => KeyCode::Numpad5,
            KP_6 => KeyCode::Numpad6,
            KP_7 => KeyCode::Numpad7,
            KP_8 => KeyCode::Numpad8,
            KP_9 => KeyCode::Numpad9,

            KP_Equal => KeyCode::NumpadEquals,
            KP_Subtract => KeyCode::NumpadSubtract,
            KP_Add => KeyCode::NumpadAdd,
            KP_Decimal => KeyCode::NumpadDecimal,
            KP_Multiply => KeyCode::NumpadMultiply,
            KP_Divide => KeyCode::NumpadDivide,
            Num_Lock => KeyCode::NumLock,
            KP_Enter => KeyCode::NumpadEnter,

            Up => KeyCode::ArrowUp,
            Down => KeyCode::ArrowDown,
            Left => KeyCode::ArrowLeft,
            Right => KeyCode::ArrowRight,
            _ => {
                log::warn!("Warning: unknown keyval {}", raw);
                KeyCode::Unknown(raw)
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
            KeyCode::Unknown(_) => unreachable!(
                "Unreachable: converting unknown KeyCode {:?} to a keyval",
                src
            ),
        }
    }
}
