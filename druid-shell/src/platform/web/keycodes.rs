// Copyright 2020 The druid Authors.
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

//! Web keycode handling.

use web_sys::KeyboardEvent;

use crate::keyboard_types::{self, Code, Key, KeyState, Location, Modifiers};

/// Convert a web-sys KeyboardEvent into a keyboard-types one.
pub(crate) fn convert_keyboard_event(event: &KeyboardEvent, modifiers: Modifiers, state: KeyState) -> keyboard_types::KeyboardEvent {
    keyboard_types::KeyboardEvent {
        state,
        key: event.key().parse().unwrap_or(Key::Unidentified),
        code: convert_code(&event.code()),
        location: convert_location(event.location()),
        modifiers,
        repeat: event.repeat(),
        is_composing: event.is_composing(),
    }
}

fn convert_code(code: &str) -> Code {
    match code {
        "Backquote" => Code::Backquote,
        "Backslash" => Code::Backslash,
        "BracketLeft" => Code::BracketLeft,
        "BracketRight" => Code::BracketRight,
        "Comma" => Code::Comma,
        "Digit0" => Code::Digit0,
        "Digit1" => Code::Digit1,
        "Digit2" => Code::Digit2,
        "Digit3" => Code::Digit3,
        "Digit4" => Code::Digit4,
        "Digit5" => Code::Digit5,
        "Digit6" => Code::Digit6,
        "Digit7" => Code::Digit7,
        "Digit8" => Code::Digit8,
        "Digit9" => Code::Digit9,
        "Equal" => Code::Equal,
        "IntlBackslash" => Code::IntlBackslash,
        "IntlRo" => Code::IntlRo,
        "IntlYen" => Code::IntlYen,
        "KeyA" => Code::KeyA,
        "KeyB" => Code::KeyB,
        "KeyC" => Code::KeyC,
        "KeyD" => Code::KeyD,
        "KeyE" => Code::KeyE,
        "KeyF" => Code::KeyF,
        "KeyG" => Code::KeyG,
        "KeyH" => Code::KeyH,
        "KeyI" => Code::KeyI,
        "KeyJ" => Code::KeyJ,
        "KeyK" => Code::KeyK,
        "KeyL" => Code::KeyL,
        "KeyM" => Code::KeyM,
        "KeyN" => Code::KeyN,
        "KeyO" => Code::KeyO,
        "KeyP" => Code::KeyP,
        "KeyQ" => Code::KeyQ,
        "KeyR" => Code::KeyR,
        "KeyS" => Code::KeyS,
        "KeyT" => Code::KeyT,
        "KeyU" => Code::KeyU,
        "KeyV" => Code::KeyV,
        "KeyW" => Code::KeyW,
        "KeyX" => Code::KeyX,
        "KeyY" => Code::KeyY,
        "KeyZ" => Code::KeyZ,
        "Minus" => Code::Minus,
        "Period" => Code::Period,
        "Quote" => Code::Quote,
        "Semicolon" => Code::Semicolon,
        "Slash" => Code::Slash,
        "AltLeft" => Code::AltLeft,
        "AltRight" => Code::AltRight,
        "Backspace" => Code::Backspace,
        "CapsLock" => Code::CapsLock,
        "ContextMenu" => Code::ContextMenu,
        "ControlLeft" => Code::ControlLeft,
        "ControlRight" => Code::ControlRight,
        "Enter" => Code::Enter,
        "MetaLeft" => Code::MetaLeft,
        "MetaRight" => Code::MetaRight,
        "ShiftLeft" => Code::ShiftLeft,
        "ShiftRight" => Code::ShiftRight,
        "Space" => Code::Space,
        "Tab" => Code::Tab,
        "Convert" => Code::Convert,
        "KanaMode" => Code::KanaMode,
        "Lang1" => Code::Lang1,
        "Lang2" => Code::Lang2,
        "Lang3" => Code::Lang3,
        "Lang4" => Code::Lang4,
        "Lang5" => Code::Lang5,
        "NonConvert" => Code::NonConvert,
        "Delete" => Code::Delete,
        "End" => Code::End,
        "Help" => Code::Help,
        "Home" => Code::Home,
        "Insert" => Code::Insert,
        "PageDown" => Code::PageDown,
        "PageUp" => Code::PageUp,
        "ArrowDown" => Code::ArrowDown,
        "ArrowLeft" => Code::ArrowLeft,
        "ArrowRight" => Code::ArrowRight,
        "ArrowUp" => Code::ArrowUp,
        "NumLock" => Code::NumLock,
        "Numpad0" => Code::Numpad0,
        "Numpad1" => Code::Numpad1,
        "Numpad2" => Code::Numpad2,
        "Numpad3" => Code::Numpad3,
        "Numpad4" => Code::Numpad4,
        "Numpad5" => Code::Numpad5,
        "Numpad6" => Code::Numpad6,
        "Numpad7" => Code::Numpad7,
        "Numpad8" => Code::Numpad8,
        "Numpad9" => Code::Numpad9,
        "NumpadAdd" => Code::NumpadAdd,
        "NumpadBackspace" => Code::NumpadBackspace,
        "NumpadClear" => Code::NumpadClear,
        "NumpadClearEntry" => Code::NumpadClearEntry,
        "NumpadComma" => Code::NumpadComma,
        "NumpadDecimal" => Code::NumpadDecimal,
        "NumpadDivide" => Code::NumpadDivide,
        "NumpadEnter" => Code::NumpadEnter,
        "NumpadEqual" => Code::NumpadEqual,
        "NumpadHash" => Code::NumpadHash,
        "NumpadMemoryAdd" => Code::NumpadMemoryAdd,
        "NumpadMemoryClear" => Code::NumpadMemoryClear,
        "NumpadMemoryRecall" => Code::NumpadMemoryRecall,
        "NumpadMemoryStore" => Code::NumpadMemoryStore,
        "NumpadMemorySubtract" => Code::NumpadMemorySubtract,
        "NumpadMultiply" => Code::NumpadMultiply,
        "NumpadParenLeft" => Code::NumpadParenLeft,
        "NumpadParenRight" => Code::NumpadParenRight,
        "NumpadStar" => Code::NumpadStar,
        "NumpadSubtract" => Code::NumpadSubtract,
        "Escape" => Code::Escape,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "Fn" => Code::Fn,
        "FnLock" => Code::FnLock,
        "PrintScreen" => Code::PrintScreen,
        "ScrollLock" => Code::ScrollLock,
        "Pause" => Code::Pause,
        "BrowserBack" => Code::BrowserBack,
        "BrowserFavorites" => Code::BrowserFavorites,
        "BrowserForward" => Code::BrowserForward,
        "BrowserHome" => Code::BrowserHome,
        "BrowserRefresh" => Code::BrowserRefresh,
        "BrowserSearch" => Code::BrowserSearch,
        "BrowserStop" => Code::BrowserStop,
        "Eject" => Code::Eject,
        "LaunchApp1" => Code::LaunchApp1,
        "LaunchApp2" => Code::LaunchApp2,
        "LaunchMail" => Code::LaunchMail,
        "MediaPlayPause" => Code::MediaPlayPause,
        "MediaSelect" => Code::MediaSelect,
        "MediaStop" => Code::MediaStop,
        "MediaTrackNext" => Code::MediaTrackNext,
        "MediaTrackPrevious" => Code::MediaTrackPrevious,
        "Power" => Code::Power,
        "Sleep" => Code::Sleep,
        "AudioVolumeDown" => Code::AudioVolumeDown,
        "AudioVolumeMute" => Code::AudioVolumeMute,
        "AudioVolumeUp" => Code::AudioVolumeUp,
        "WakeUp" => Code::WakeUp,
        "Hyper" => Code::Hyper,
        "Super" => Code::Super,
        "Turbo" => Code::Turbo,
        "Abort" => Code::Abort,
        "Resume" => Code::Resume,
        "Suspend" => Code::Suspend,
        "Again" => Code::Again,
        "Copy" => Code::Copy,
        "Cut" => Code::Cut,
        "Find" => Code::Find,
        "Open" => Code::Open,
        "Paste" => Code::Paste,
        "Props" => Code::Props,
        "Select" => Code::Select,
        "Undo" => Code::Undo,
        "Hiragana" => Code::Hiragana,
        "Katakana" => Code::Katakana,
        // Should be exhaustive but in case not, use reasonable default
        _ => Code::Unidentified,
    }
}

fn convert_location(loc: u32) -> Location {
    match loc {
        KeyboardEvent::DOM_KEY_LOCATION_LEFT => Location::Left,
        KeyboardEvent::DOM_KEY_LOCATION_RIGHT => Location::Right,
        KeyboardEvent::DOM_KEY_LOCATION_NUMPAD => Location::Numpad,
        // Should be exhaustive but in case not, use reasonable default
        _ => Location::Standard,
    }
}

/*
// TODO: use macro in window to avoid code dupl
fn get_modifiers(event: &KeyboardEvent) -> Modifiers {
    let mut result = Modifiers::default();
    result.set(Modifiers::SHIFT, event.shift_key());
    result.set(Modifiers::ALT, event.alt_key());
    result.set(Modifiers::CONTROL, event.ctrl_key());
    result.set(Modifiers::META, event.meta_key());
    result.set(Modifiers::ALT_GRAPH, event.get_modifier_state("AltGraph"));
    result.set(Modifiers::CAPS_LOCK, event.get_modifier_state("CapsLock"));
    result.set(Modifiers::NUM_LOCK, event.get_modifier_state("NumLock"));
    result.set(Modifiers::SCROLL_LOCK, event.get_modifier_state("ScrollLock"));
    result
}
*/

/*
macro_rules! map_keys {
    ($( ($id:path, ($loc:tt)) => $code:path),*) => {
        impl From<KeyCode> for u32 {
            fn from(src: KeyCode) -> u32 {
                match src {
                    $(
                        $code => $id
                    ),*,
                    other => {
                        log::warn!("Unrecognized druid KeyCode: {:?}", other);
                        u32::max_value()
                    }
                }
            }
        }

        // The first number is the keycode and the second is the location
        impl From<(u32, u32)> for KeyCode {
            fn from(src: (u32, u32)) -> KeyCode {
                match src {
                    $(
                        ($id, $loc) => $code
                    ),*,
                    other => KeyCode::Unknown(other.0),
                }
            }
        }
    }
}

map_keys! {
    //(KeyEvent::DOM_VK_CANCEL, (_)) => KeyCode::Cancel,
    //(KeyEvent::DOM_VK_HELP, (_)) => KeyCode::Help,
    (KeyEvent::DOM_VK_BACK_SPACE, (_)) => KeyCode::Backspace,
    (KeyEvent::DOM_VK_TAB, (_)) => KeyCode::Tab,
    //(KeyEvent::DOM_VK_CLEAR, (_)) => KeyCode::NumLock,
    (KeyEvent::DOM_VK_RETURN, (_)) => KeyCode::Return,
    (KeyEvent::DOM_VK_SHIFT, (LOC_LEFT)) => KeyCode::LeftShift,
    (KeyEvent::DOM_VK_SHIFT, (LOC_RIGHT)) => KeyCode::RightShift,
    (KeyEvent::DOM_VK_CONTROL, (LOC_LEFT)) => KeyCode::LeftControl,
    (KeyEvent::DOM_VK_CONTROL, (LOC_RIGHT)) => KeyCode::RightControl,
    (KeyEvent::DOM_VK_ALT, (LOC_LEFT)) => KeyCode::LeftAlt,
    (KeyEvent::DOM_VK_ALT, (LOC_RIGHT)) => KeyCode::RightAlt,
    (KeyEvent::DOM_VK_PAUSE, (_)) => KeyCode::Pause,
    (KeyEvent::DOM_VK_CAPS_LOCK, (_)) => KeyCode::CapsLock,
    //(KeyEvent::DOM_VK_KANA, (_)) => KeyCode::KANA,
    //(KeyEvent::DOM_VK_HANGUL, (_)) => KeyCode::HANGUL,
    //(KeyEvent::DOM_VK_EISU, (_)) => KeyCode::EISU,
    //(KeyEvent::DOM_VK_JUNJA, (_)) => KeyCode::JUNJA,
    //(KeyEvent::DOM_VK_FINAL, (_)) => KeyCode::FINAL,
    //(KeyEvent::DOM_VK_HANJA, (_)) => KeyCode::HANJA,
    //(KeyEvent::DOM_VK_KANJI, (_)) => KeyCode::KANJI,
    (KeyEvent::DOM_VK_ESCAPE, (_)) => KeyCode::Escape,
    //(KeyEvent::DOM_VK_CONVERT, (_)) => KeyCode::CONVERT,
    //(KeyEvent::DOM_VK_NONCONVERT, (_)) => KeyCode::NONCONVERT,
    //(KeyEvent::DOM_VK_ACCEPT, (_)) => KeyCode::ACCEPT,
    //(KeyEvent::DOM_VK_MODECHANGE, (_)) => KeyCode::MODECHANGE,
    (KeyEvent::DOM_VK_SPACE, (_)) => KeyCode::Space,
    (KeyEvent::DOM_VK_PAGE_UP, (_)) => KeyCode::PageUp,
    (KeyEvent::DOM_VK_PAGE_DOWN, (_)) => KeyCode::PageDown,
    (KeyEvent::DOM_VK_END, (_)) => KeyCode::End,
    (KeyEvent::DOM_VK_HOME, (_)) => KeyCode::Home,
    (KeyEvent::DOM_VK_LEFT, (_)) => KeyCode::ArrowLeft,
    (KeyEvent::DOM_VK_UP, (_)) => KeyCode::ArrowUp,
    (KeyEvent::DOM_VK_RIGHT, (_)) => KeyCode::ArrowRight,
    (KeyEvent::DOM_VK_DOWN, (_)) => KeyCode::ArrowDown,
    //(KeyEvent::DOM_VK_SELECT, (_)) => KeyCode::SELECT,
    //(KeyEvent::DOM_VK_PRINT, (_)) => KeyCode::PrintScreen,
    //(KeyEvent::DOM_VK_EXECUTE, (_)) => KeyCode::EXECUTE,
    (KeyEvent::DOM_VK_PRINTSCREEN, (_)) => KeyCode::PrintScreen,
    (KeyEvent::DOM_VK_INSERT, (_)) => KeyCode::Insert,
    (KeyEvent::DOM_VK_DELETE, (_)) => KeyCode::Delete,
    (KeyEvent::DOM_VK_0, (_)) => KeyCode::Key0,
    (KeyEvent::DOM_VK_1, (_)) => KeyCode::Key1,
    (KeyEvent::DOM_VK_2, (_)) => KeyCode::Key2,
    (KeyEvent::DOM_VK_3, (_)) => KeyCode::Key3,
    (KeyEvent::DOM_VK_4, (_)) => KeyCode::Key4,
    (KeyEvent::DOM_VK_5, (_)) => KeyCode::Key5,
    (KeyEvent::DOM_VK_6, (_)) => KeyCode::Key6,
    (KeyEvent::DOM_VK_7, (_)) => KeyCode::Key7,
    (KeyEvent::DOM_VK_8, (_)) => KeyCode::Key8,
    (KeyEvent::DOM_VK_9, (_)) => KeyCode::Key9,
    //(KeyEvent::DOM_VK_COLON, (_)) => KeyCode::Semicolon,
    (KeyEvent::DOM_VK_SEMICOLON, (_)) => KeyCode::Semicolon,
    //(KeyEvent::DOM_VK_LESS_THAN, (_)) => KeyCode::Comma,
    (KeyEvent::DOM_VK_EQUALS, (_)) => KeyCode::Equals,
    //(KeyEvent::DOM_VK_GREATER_THAN, (_)) => KeyCode::Period,
    //(KeyEvent::DOM_VK_QUESTION_MARK, (_)) => KeyCode::Slash,
    //(KeyEvent::DOM_VK_AT, (_)) => KeyCode::AT,
    (KeyEvent::DOM_VK_A, (_)) => KeyCode::KeyA,
    (KeyEvent::DOM_VK_B, (_)) => KeyCode::KeyB,
    (KeyEvent::DOM_VK_C, (_)) => KeyCode::KeyC,
    (KeyEvent::DOM_VK_D, (_)) => KeyCode::KeyD,
    (KeyEvent::DOM_VK_E, (_)) => KeyCode::KeyE,
    (KeyEvent::DOM_VK_F, (_)) => KeyCode::KeyF,
    (KeyEvent::DOM_VK_G, (_)) => KeyCode::KeyG,
    (KeyEvent::DOM_VK_H, (_)) => KeyCode::KeyH,
    (KeyEvent::DOM_VK_I, (_)) => KeyCode::KeyI,
    (KeyEvent::DOM_VK_J, (_)) => KeyCode::KeyJ,
    (KeyEvent::DOM_VK_K, (_)) => KeyCode::KeyK,
    (KeyEvent::DOM_VK_L, (_)) => KeyCode::KeyL,
    (KeyEvent::DOM_VK_M, (_)) => KeyCode::KeyM,
    (KeyEvent::DOM_VK_N, (_)) => KeyCode::KeyN,
    (KeyEvent::DOM_VK_O, (_)) => KeyCode::KeyO,
    (KeyEvent::DOM_VK_P, (_)) => KeyCode::KeyP,
    (KeyEvent::DOM_VK_Q, (_)) => KeyCode::KeyQ,
    (KeyEvent::DOM_VK_R, (_)) => KeyCode::KeyR,
    (KeyEvent::DOM_VK_S, (_)) => KeyCode::KeyS,
    (KeyEvent::DOM_VK_T, (_)) => KeyCode::KeyT,
    (KeyEvent::DOM_VK_U, (_)) => KeyCode::KeyU,
    (KeyEvent::DOM_VK_V, (_)) => KeyCode::KeyV,
    (KeyEvent::DOM_VK_W, (_)) => KeyCode::KeyW,
    (KeyEvent::DOM_VK_X, (_)) => KeyCode::KeyX,
    (KeyEvent::DOM_VK_Y, (_)) => KeyCode::KeyY,
    (KeyEvent::DOM_VK_Z, (_)) => KeyCode::KeyZ,
    //(KeyEvent::DOM_VK_WIN, (LOC_LEFT)) => KeyCode::LeftMeta,
    //(KeyEvent::DOM_VK_WIN, (LOC_RIGHT)) => KeyCode::RightMeta,
    //(KeyEvent::DOM_VK_CONTEXT_MENU, (_)) => KeyCode::CONTEXT_MENU,
    //(KeyEvent::DOM_VK_SLEEP, (_)) => KeyCode::SLEEP,
    (KeyEvent::DOM_VK_NUMPAD0, (_)) => KeyCode::Numpad0,
    (KeyEvent::DOM_VK_NUMPAD1, (_)) => KeyCode::Numpad1,
    (KeyEvent::DOM_VK_NUMPAD2, (_)) => KeyCode::Numpad2,
    (KeyEvent::DOM_VK_NUMPAD3, (_)) => KeyCode::Numpad3,
    (KeyEvent::DOM_VK_NUMPAD4, (_)) => KeyCode::Numpad4,
    (KeyEvent::DOM_VK_NUMPAD5, (_)) => KeyCode::Numpad5,
    (KeyEvent::DOM_VK_NUMPAD6, (_)) => KeyCode::Numpad6,
    (KeyEvent::DOM_VK_NUMPAD7, (_)) => KeyCode::Numpad7,
    (KeyEvent::DOM_VK_NUMPAD8, (_)) => KeyCode::Numpad8,
    (KeyEvent::DOM_VK_NUMPAD9, (_)) => KeyCode::Numpad9,
    (KeyEvent::DOM_VK_MULTIPLY, (LOC_NUMPAD)) => KeyCode::NumpadMultiply,
    (KeyEvent::DOM_VK_ADD, (LOC_NUMPAD)) => KeyCode::NumpadAdd,
    //(KeyEvent::DOM_VK_SEPARATOR, (_)) => KeyCode::SEPARATOR,
    (KeyEvent::DOM_VK_SUBTRACT, (LOC_NUMPAD)) => KeyCode::NumpadSubtract,
    (KeyEvent::DOM_VK_DECIMAL, (LOC_NUMPAD)) => KeyCode::NumpadDecimal,
    (KeyEvent::DOM_VK_DIVIDE, (LOC_NUMPAD)) => KeyCode::NumpadDivide,
    (KeyEvent::DOM_VK_F1, (_)) => KeyCode::F1,
    (KeyEvent::DOM_VK_F2, (_)) => KeyCode::F2,
    (KeyEvent::DOM_VK_F3, (_)) => KeyCode::F3,
    (KeyEvent::DOM_VK_F4, (_)) => KeyCode::F4,
    (KeyEvent::DOM_VK_F5, (_)) => KeyCode::F5,
    (KeyEvent::DOM_VK_F6, (_)) => KeyCode::F6,
    (KeyEvent::DOM_VK_F7, (_)) => KeyCode::F7,
    (KeyEvent::DOM_VK_F8, (_)) => KeyCode::F8,
    (KeyEvent::DOM_VK_F9, (_)) => KeyCode::F9,
    (KeyEvent::DOM_VK_F10, (_)) => KeyCode::F10,
    (KeyEvent::DOM_VK_F11, (_)) => KeyCode::F11,
    (KeyEvent::DOM_VK_F12, (_)) => KeyCode::F12,
    //(KeyEvent::DOM_VK_F13, (_)) => KeyCode::F13,
    //(KeyEvent::DOM_VK_F14, (_)) => KeyCode::F14,
    //(KeyEvent::DOM_VK_F15, (_)) => KeyCode::F15,
    //(KeyEvent::DOM_VK_F16, (_)) => KeyCode::F16,
    //(KeyEvent::DOM_VK_F17, (_)) => KeyCode::F17,
    //(KeyEvent::DOM_VK_F18, (_)) => KeyCode::F18,
    //(KeyEvent::DOM_VK_F19, (_)) => KeyCode::F19,
    //(KeyEvent::DOM_VK_F20, (_)) => KeyCode::F20,
    //(KeyEvent::DOM_VK_F21, (_)) => KeyCode::F21,
    //(KeyEvent::DOM_VK_F22, (_)) => KeyCode::F22,
    //(KeyEvent::DOM_VK_F23, (_)) => KeyCode::F23,
    //(KeyEvent::DOM_VK_F24, (_)) => KeyCode::F24,
    (KeyEvent::DOM_VK_NUM_LOCK, (_)) => KeyCode::NumLock,
    (KeyEvent::DOM_VK_SCROLL_LOCK, (_)) => KeyCode::ScrollLock,
    //(KeyEvent::DOM_VK_WIN_OEM_FJ_JISHO, (_)) => KeyCode::WIN_OEM_FJ_JISHO,
    //(KeyEvent::DOM_VK_WIN_OEM_FJ_MASSHOU, (_)) => KeyCode::WIN_OEM_FJ_MASSHOU,
    //(KeyEvent::DOM_VK_WIN_OEM_FJ_TOUROKU, (_)) => KeyCode::WIN_OEM_FJ_TOUROKU,
    //(KeyEvent::DOM_VK_WIN_OEM_FJ_LOYA, (_)) => KeyCode::WIN_OEM_FJ_LOYA,
    //(KeyEvent::DOM_VK_WIN_OEM_FJ_ROYA, (_)) => KeyCode::WIN_OEM_FJ_ROYA,
    //(KeyEvent::DOM_VK_CIRCUMFLEX, (_)) => KeyCode::CIRCUMFLEX,
    //(KeyEvent::DOM_VK_EXCLAMATION, (_)) => KeyCode::Key1,
    //(KeyEvent::DOM_VK_DOUBLE_QUOTE, (_)) => KeyCode::Quote,
    //(KeyEvent::DOM_VK_HASH, (_)) => KeyCode::Key3,
    //(KeyEvent::DOM_VK_DOLLAR, (_)) => KeyCode::Key4,
    //(KeyEvent::DOM_VK_PERCENT, (_)) => KeyCode::Key5,
    //(KeyEvent::DOM_VK_AMPERSAND, (_)) => KeyCode::Key7,
    //(KeyEvent::DOM_VK_UNDERSCORE, (_)) => KeyCode::Minus,
    //(KeyEvent::DOM_VK_OPEN_PAREN, (_)) => KeyCode::Key9,
    //(KeyEvent::DOM_VK_CLOSE_PAREN, (_)) => KeyCode::Key0,
    //(KeyEvent::DOM_VK_ASTERISK, (_)) => KeyCode::Key8,
    //(KeyEvent::DOM_VK_PLUS, (_)) => KeyCode::Equals,
    //(KeyEvent::DOM_VK_PIPE, (_)) => KeyCode::Backslash,
    (KeyEvent::DOM_VK_HYPHEN_MINUS, (_)) => KeyCode::Minus,
    //(KeyEvent::DOM_VK_OPEN_CURLY_BRACKET, (_)) => KeyCode::LeftBracket,
    //(KeyEvent::DOM_VK_CLOSE_CURLY_BRACKET, (_)) => KeyCode::RightBracket,
    (KeyEvent::DOM_VK_TILDE, (_)) => KeyCode::Backtick,
    //(KeyEvent::DOM_VK_VOLUME_MUTE, (_)) => KeyCode::VOLUME_MUTE,
    //(KeyEvent::DOM_VK_VOLUME_DOWN, (_)) => KeyCode::VOLUME_DOWN,
    //(KeyEvent::DOM_VK_VOLUME_UP, (_)) => KeyCode::VOLUME_UP,
    (KeyEvent::DOM_VK_COMMA, (_)) => KeyCode::Comma,
    (KeyEvent::DOM_VK_PERIOD, (_)) => KeyCode::Period,
    (KeyEvent::DOM_VK_SLASH, (_)) => KeyCode::Slash,
    //(KeyEvent::DOM_VK_BACK_QUOTE, (_)) => KeyCode::Quote,
    (KeyEvent::DOM_VK_OPEN_BRACKET, (_)) => KeyCode::LeftBracket,
    (KeyEvent::DOM_VK_BACK_SLASH, (_)) => KeyCode::Backslash,
    (KeyEvent::DOM_VK_CLOSE_BRACKET, (_)) => KeyCode::RightBracket,
    (KeyEvent::DOM_VK_QUOTE, (_)) => KeyCode::Quote,
    (KeyEvent::DOM_VK_META, (LOC_LEFT)) => KeyCode::LeftMeta,
    (KeyEvent::DOM_VK_META, (LOC_RIGHT)) => KeyCode::RightMeta
    //(KeyEvent::DOM_VK_ALTGR, (_)) => KeyCode::RightAlt,
    //(KeyEvent::DOM_VK_WIN_ICO_HELP, (_)) => KeyCode::WIN_ICO_HELP,
    //(KeyEvent::DOM_VK_WIN_ICO_00, (_)) => KeyCode::WIN_ICO_00,
    //(KeyEvent::DOM_VK_PROCESSKEY, (_)) => KeyCode::PROCESSKEY,
    //(KeyEvent::DOM_VK_WIN_ICO_CLEAR, (_)) => KeyCode::WIN_ICO_CLEAR,
    //(KeyEvent::DOM_VK_WIN_OEM_RESET, (_)) => KeyCode::WIN_OEM_RESET,
    //(KeyEvent::DOM_VK_WIN_OEM_JUMP, (_)) => KeyCode::WIN_OEM_JUMP,
    //(KeyEvent::DOM_VK_WIN_OEM_PA1, (_)) => KeyCode::WIN_OEM_PA1,
    //(KeyEvent::DOM_VK_WIN_OEM_PA2, (_)) => KeyCode::WIN_OEM_PA2,
    //(KeyEvent::DOM_VK_WIN_OEM_PA3, (_)) => KeyCode::WIN_OEM_PA3,
    //(KeyEvent::DOM_VK_WIN_OEM_WSCTRL, (_)) => KeyCode::WIN_OEM_WSCTRL,
    //(KeyEvent::DOM_VK_WIN_OEM_CUSEL, (_)) => KeyCode::WIN_OEM_CUSEL,
    //(KeyEvent::DOM_VK_WIN_OEM_ATTN, (_)) => KeyCode::WIN_OEM_ATTN,
    //(KeyEvent::DOM_VK_WIN_OEM_FINISH, (_)) => KeyCode::WIN_OEM_FINISH,
    //(KeyEvent::DOM_VK_WIN_OEM_COPY, (_)) => KeyCode::WIN_OEM_COPY,
    //(KeyEvent::DOM_VK_WIN_OEM_AUTO, (_)) => KeyCode::WIN_OEM_AUTO,
    //(KeyEvent::DOM_VK_WIN_OEM_ENLW, (_)) => KeyCode::WIN_OEM_ENLW,
    //(KeyEvent::DOM_VK_WIN_OEM_BACKTAB, (_)) => KeyCode::WIN_OEM_BACKTAB,
    //(KeyEvent::DOM_VK_ATTN, (_)) => KeyCode::ATTN,
    //(KeyEvent::DOM_VK_CRSEL, (_)) => KeyCode::CRSEL,
    //(KeyEvent::DOM_VK_EXSEL, (_)) => KeyCode::EXSEL,
    //(KeyEvent::DOM_VK_EREOF, (_)) => KeyCode::EREOF,
    //(KeyEvent::DOM_VK_PLAY, (_)) => KeyCode::PLAY,
    //(KeyEvent::DOM_VK_ZOOM, (_)) => KeyCode::ZOOM,
    //(KeyEvent::DOM_VK_PA1, (_)) => KeyCode::PA1,
    //(KeyEvent::DOM_VK_WIN_OEM_CLEAR, (_)) => KeyCode::WIN_OEM_CLEAR,
}

/// A helper to convert the key string to the text that it is supposed to represent.
pub(crate) fn key_to_text(key: &str) -> &str {
    // The following list was taken from
    // https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values
    match key {
        // Whitespace
        "Enter" => "\n",
        "Tab" => "\t",

        // Numpad Keys
        "Decimal" => ".",
        "Multiply" => "*",
        "Add" => "+",
        "Divide" => "/",
        "Subtract" => "-",

        // Special
        "Unidentified"
        // Modifiers
        | "Alt"
        | "AltGraph"
        | "CapsLock"
        | "Control"
        | "Fn"
        | "FnLock"
        | "Hyper"
        | "Meta"
        | "NumLock"
        | "ScrollLock"
        | "Shift"
        | "Super"
        | "Symbol"
        | "SymbolLock"
        // Navigation
        | "ArrowDown"
        | "ArrowLeft"
        | "ArrowRight"
        | "ArrowUp"
        | "End"
        | "Home"
        | "PageDown"
        | "PageUp"
        // Editing
        | "Backspace"
        | "Clear"
        | "Copy"
        | "CrSel"
        | "Cut"
        | "Delete"
        | "EraseEof"
        | "ExSel"
        | "Insert"
        | "Paste"
        | "Redo"
        | "Undo"
        // UI
        | "Accept"
        | "Again"
        | "Attn"
        | "Cancel"
        | "ContextMenu"
        | "Escape"
        | "Execute"
        | "Find"
        | "Finish"
        | "Help"
        | "Pause"
        | "Play"
        | "Props"
        | "Select"
        | "ZoomIn"
        | "ZoomOut"
        // Device
        | "BrightnessDown"
        | "BrightnessUp"
        | "Eject"
        | "LogOff"
        | "Power"
        | "PowerOff"
        | "PrintScreen"
        | "Hibernate"
        | "Standby"
        | "WakeUp"
        // IME
        | "AllCandidates"
        | "Alphanumeric"
        | "CodeInput"
        | "Compose"
        | "Convert"
        | "Dead"
        | "FinalMode"
        | "GroupFirst"
        | "GroupLast"
        | "GroupNext"
        | "GroupPrevious"
        | "ModeChange"
        | "NextCandidate"
        | "NonConvert"
        | "PreviousCandidate"
        | "Process"
        | "SingleCandidate" 
        // Korean
        | "HangulMode"
        | "HanjaMode"
        | "JunjaMode"
        // Japanese
        | "Eisu"
        | "Hankaku"
        | "Hiragana"
        | "HiraganaKatakana"
        | "KanaMode"
        | "KanjiMode"
        | "Katakana"
        | "Romaji"
        | "Zenkaku"
        | "ZenkakuHanaku" 
        // Function
        | "F1"
        | "F2"
        | "F3"
        | "F4"
        | "F5"
        | "F6"
        | "F7"
        | "F8"
        | "F9"
        | "F10"
        | "F11"
        | "F12"
        | "F13"
        | "F14"
        | "F15"
        | "F16"
        | "F17"
        | "F18"
        | "F19"
        | "F20"
        | "Soft1"
        | "Soft2"
        | "Soft3"
        | "Soft4"
        // Phone
        | "AppSwitch"
        | "Call"
        | "Camera"
        | "CameraFocus"
        | "EndCall"
        | "GoBack"
        | "GoHome"
        | "HeadsetHook"
        | "LastNumberRedial"
        | "Notification"
        | "MannerMode"
        | "VoiceDial"
        // Multimedia
        | "ChannelDown"
        | "ChannelUp"
        | "MediaFastForward"
        | "MediaPause"
        | "MediaPlay"
        | "MediaPlayPause"
        | "MediaRecord"
        | "MediaRewind"
        | "MediaStop"
        | "MediaTrackNext"
        | "MediaTrackPrevious"
        // Audio
        | "AudioBalanceLeft"
        | "AudioBalanceRight"
        | "AudioBassDown"
        | "AudioBassBoostDown"
        | "AudioBassBoostToggle"
        | "AudioBassBoostUp"
        | "AudioBassUp"
        | "AudioFaderFront"
        | "AudioFaderRear"
        | "AudioSurroundModeNext"
        | "AudioTrebleDown"
        | "AudioTrebleUp"
        | "AudioVolumeDown"
        | "AudioVolumeMute"
        | "AudioVolumeUp"
        | "MicrophoneToggle"
        | "MicrophoneVolumeDown"
        | "MicrophoneVolumeMute"
        | "MicrophoneVolumeUp"
        // TV control
        | "TV"
        | "TV3DMode"
        | "TVAntennaCable"
        | "TVAudioDescription"
        | "TVAudioDescriptionMixDown"
        | "TVAudioDescriptionMixUp"
        | "TVContentsMenu"
        | "TVDataService"
        | "TVInput"
        | "TVInputComponent1"
        | "TVInputComponent2"
        | "TVInputComposite1"
        | "TVInputComposite2"
        | "TVInputHDMI1"
        | "TVInputHDMI2"
        | "TVInputHDMI3"
        | "TVInputHDMI4"
        | "TVInputVGA1"
        | "TVMediaContext"
        | "TVNetwork"
        | "TVNumberEntry"
        | "TVPower"
        | "TVRadioService"
        | "TVSatellite"
        | "TVSatelliteBS"
        | "TVSatelliteCS"
        | "TVSatelliteToggle"
        | "TVTerrestrialAnalog"
        | "TVTerrestrialDigital"
        | "TVTimer"
        // Media Control (Possibly repeated)
        | "AVRInput"
        | "AVRPower"
        | "ColorF0Red"
        | "ColorF1Green"
        | "ColorF2Yellow"
        | "ColorF3Blue"
        | "ColorF4Grey"
        | "ColorF5Brown"
        | "ClosedCaptionToggle"
        | "Dimmer"
        | "DisplaySwap"
        | "DVR"
        | "Exit"
        | "FavoriteClear0"
        | "FavoriteClear1"
        | "FavoriteClear2"
        | "FavoriteClear3"
        | "FavoriteRecall0"
        | "FavoriteRecall1"
        | "FavoriteRecall2"
        | "FavoriteRecall3"
        | "FavoriteStore0"
        | "FavoriteStore1"
        | "FavoriteStore2"
        | "FavoriteStore3"
        | "Guide"
        | "GuideNextDay"
        | "GuidePreviousDay"
        | "Info"
        | "InstantReplay"
        | "Link"
        | "ListProgram"
        | "LiveContent"
        | "Lock"
        | "MediaApps"
        | "MediaAudioTrack"
        | "MediaLast"
        | "MediaSkipBackward"
        | "MediaSkipForward"
        | "MediaStepBackward"
        | "MediaStepForward"
        | "MediaTopMenu"
        | "NavigateIn"
        | "NavigateNext"
        | "NavigateOut"
        | "NavigatePrevious"
        | "NextFavoriteChannel"
        | "NextUserProfile"
        | "OnDemand"
        | "Pairing"
        | "PinPDown"
        | "PinPMove"
        | "PinPToggle"
        | "PinPUp"
        | "PlaySpeedDown"
        | "PlaySpeedReset"
        | "PlaySpeedUp"
        | "RandomToggle"
        | "RcLowBattery"
        | "RecordSpeedNext"
        | "RfBypass"
        | "ScanChannelsToggle"
        | "ScreenModeNext"
        | "Settings"
        | "SplitScreenToggle"
        | "STBInput"
        | "STBPower"
        | "Subtitle"
        | "Teletext"
        | "VideoModeNext"
        | "Wink"
        | "ZoomToggle" 
        // Speech recognition
        | "SpeechCorrectionList"
        | "SpeechInputToggle"
        // Document
        | "Close"
        | "New"
        | "Open"
        | "Print"
        | "Save"
        | "SpellCheck"
        | "MailForward"
        | "MailReply"
        | "MailSend"
        // Application selector
        | "LaunchCalculator"
        | "LaunchCalendar"
        | "LaunchContacts"
        | "LaunchMail"
        | "LaunchMediaPlayer"
        | "LaunchMusicPlayer"
        | "LaunchMyComputer"
        | "LaunchPhone"
        | "LaunchScreenSaver"
        | "LaunchSpreadsheet"
        | "LaunchWebBrowser"
        | "LaunchWebCam"
        | "LaunchWordProcessor"
        | "LaunchApplication1"
        | "LaunchApplication2"
        | "LaunchApplication3"
        | "LaunchApplication4"
        | "LaunchApplication5"
        | "LaunchApplication6"
        | "LaunchApplication7"
        | "LaunchApplication8"
        | "LaunchApplication9"
        | "LaunchApplication10"
        | "LaunchApplication11"
        | "LaunchApplication12"
        | "LaunchApplication13"
        | "LaunchApplication14"
        | "LaunchApplication15"
        | "LaunchApplication16"
        // Browser Control
        | "BrowserBack"
        | "BrowserFavorites"
        | "BrowserForward"
        | "BrowserHome"
        | "BrowserRefresh"
        | "BrowserSearch"
        | "BrowserStop"
        // Numeric keypad
        | "Key11"
        | "Key12" 
        | "Separator" => "",

        k => k,
    }
}
*/
