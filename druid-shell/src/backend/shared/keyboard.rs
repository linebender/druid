// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Keyboard logic that is shared by more than one backend.

#[allow(unused)]
use keyboard_types::{Code, Location};

#[cfg(any(
    all(
        any(feature = "x11", feature = "wayland"),
        any(target_os = "freebsd", target_os = "linux", target_os = "openbsd")
    ),
    target_os = "macos"
))]
/// Map key code to location.
///
/// The logic for this is adapted from InitKeyEvent in TextInputHandler (in the Mozilla
/// mac port).
///
/// Note: in the original, this is based on kVK constants, but since we don't have those
/// readily available, we use the mapping to code (which should be effectively lossless).
pub fn code_to_location(code: Code) -> Location {
    match code {
        Code::MetaLeft | Code::ShiftLeft | Code::AltLeft | Code::ControlLeft => Location::Left,
        Code::MetaRight | Code::ShiftRight | Code::AltRight | Code::ControlRight => Location::Right,
        Code::Numpad0
        | Code::Numpad1
        | Code::Numpad2
        | Code::Numpad3
        | Code::Numpad4
        | Code::Numpad5
        | Code::Numpad6
        | Code::Numpad7
        | Code::Numpad8
        | Code::Numpad9
        | Code::NumpadAdd
        | Code::NumpadComma
        | Code::NumpadDecimal
        | Code::NumpadDivide
        | Code::NumpadEnter
        | Code::NumpadEqual
        | Code::NumpadMultiply
        | Code::NumpadSubtract => Location::Numpad,
        _ => Location::Standard,
    }
}

#[cfg(any(target_os = "freebsd", target_os = "linux", target_os = "openbsd"))]
/// Map hardware keycode to code.
///
/// In theory, the hardware keycode is device dependent, but in
/// practice it's probably pretty reliable.
///
/// The logic is based on NativeKeyToDOMCodeName.h in Mozilla.
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
        0x005D => Code::Lang5,
        0x005E => Code::IntlBackslash,
        0x005F => Code::F11,
        0x0060 => Code::F12,
        0x0061 => Code::IntlRo,
        0x0062 => Code::Lang3,
        0x0063 => Code::Lang4,
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
        0x007C => Code::Power,
        0x007D => Code::NumpadEqual,
        0x007F => Code::Pause,
        0x0080 => Code::ShowAllWindows,
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
        0x0096 => Code::Sleep,
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
        0x00AF => Code::MediaRecord,
        0x00B0 => Code::MediaRewind,
        0x00B3 => Code::MediaSelect,
        0x00B4 => Code::BrowserHome,
        0x00B5 => Code::BrowserRefresh,
        0x00BB => Code::NumpadParenLeft,
        0x00BC => Code::NumpadParenRight,
        0x00BF => Code::F13,
        0x00C0 => Code::F14,
        0x00C1 => Code::F15,
        0x00C2 => Code::F16,
        0x00C3 => Code::F17,
        0x00C4 => Code::F18,
        0x00C5 => Code::F19,
        0x00C6 => Code::F20,
        0x00C7 => Code::F21,
        0x00C8 => Code::F22,
        0x00C9 => Code::F23,
        0x00CA => Code::F24,
        0x00D1 => Code::MediaPause,
        0x00D7 => Code::MediaPlay,
        0x00D8 => Code::MediaFastForward,
        0x00E1 => Code::BrowserSearch,
        0x00E8 => Code::BrightnessDown,
        0x00E9 => Code::BrightnessUp,
        0x00EB => Code::DisplayToggleIntExt,
        0x00EF => Code::MailSend,
        0x00F0 => Code::MailReply,
        0x00F1 => Code::MailForward,
        0x0100 => Code::MicrophoneMuteToggle,
        0x017C => Code::ZoomToggle,
        0x024B => Code::LaunchControlPanel,
        0x024C => Code::SelectTask,
        0x024D => Code::LaunchScreenSaver,
        0x024F => Code::LaunchAssistant,
        0x0250 => Code::KeyboardLayoutSelect,
        0x0281 => Code::PrivacyScreenToggle,
        _ => Code::Unidentified,
    }
}
