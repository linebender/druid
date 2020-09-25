// Copyright 2020 The Druid Authors.
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

//! Key event handling.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::mem;
use std::ops::RangeInclusive;

use crate::keyboard::{Code, KbKey, KeyEvent, KeyState, Location, Modifiers};

use winapi::shared::minwindef::{HKL, INT, LPARAM, UINT, WPARAM};
use winapi::shared::ntdef::SHORT;
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    GetKeyState, GetKeyboardLayout, MapVirtualKeyExW, PeekMessageW, ToUnicodeEx, VkKeyScanW,
    MAPVK_VK_TO_CHAR, MAPVK_VSC_TO_VK_EX, PM_NOREMOVE, VK_ACCEPT, VK_ADD, VK_APPS, VK_ATTN,
    VK_BACK, VK_BROWSER_BACK, VK_BROWSER_FAVORITES, VK_BROWSER_FORWARD, VK_BROWSER_HOME,
    VK_BROWSER_REFRESH, VK_BROWSER_SEARCH, VK_BROWSER_STOP, VK_CANCEL, VK_CAPITAL, VK_CLEAR,
    VK_CONTROL, VK_CONVERT, VK_CRSEL, VK_DECIMAL, VK_DELETE, VK_DIVIDE, VK_DOWN, VK_END, VK_EREOF,
    VK_ESCAPE, VK_EXECUTE, VK_EXSEL, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5,
    VK_F6, VK_F7, VK_F8, VK_F9, VK_FINAL, VK_HELP, VK_HOME, VK_INSERT, VK_JUNJA, VK_KANA, VK_KANJI,
    VK_LAUNCH_APP1, VK_LAUNCH_APP2, VK_LAUNCH_MAIL, VK_LAUNCH_MEDIA_SELECT, VK_LCONTROL, VK_LEFT,
    VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK,
    VK_MEDIA_STOP, VK_MENU, VK_MODECHANGE, VK_MULTIPLY, VK_NEXT, VK_NONCONVERT, VK_NUMLOCK,
    VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7,
    VK_NUMPAD8, VK_NUMPAD9, VK_OEM_ATTN, VK_OEM_CLEAR, VK_PAUSE, VK_PLAY, VK_PRINT, VK_PRIOR,
    VK_PROCESSKEY, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL,
    VK_SELECT, VK_SHIFT, VK_SLEEP, VK_SNAPSHOT, VK_SUBTRACT, VK_TAB, VK_UP, VK_VOLUME_DOWN,
    VK_VOLUME_MUTE, VK_VOLUME_UP, VK_ZOOM, WM_CHAR, WM_INPUTLANGCHANGE, WM_KEYDOWN, WM_KEYUP,
    WM_SYSCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const VK_ABNT_C2: INT = 0xc2;

/// A (non-extended) virtual key code.
type VkCode = u8;

// This is really bitfields.
type ShiftState = u8;
const SHIFT_STATE_SHIFT: ShiftState = 1;
const SHIFT_STATE_ALTGR: ShiftState = 2;
const N_SHIFT_STATE: ShiftState = 4;

/// Per-window keyboard state.
pub(crate) struct KeyboardState {
    hkl: HKL,
    // A map from (vk, is_shifted) to string val
    key_vals: HashMap<(VkCode, ShiftState), String>,
    dead_keys: HashSet<(VkCode, ShiftState)>,
    has_altgr: bool,
    stash_vk: Option<VkCode>,
    stash_utf16: Vec<u16>,
}

/// Virtual key codes that are considered printable.
///
/// This logic is borrowed from KeyboardLayout::GetKeyIndex
/// in Mozilla.
const PRINTABLE_VKS: &[RangeInclusive<VkCode>] = &[
    0x20..=0x20,
    0x30..=0x39,
    0x41..=0x5A,
    0x60..=0x6B,
    0x6D..=0x6F,
    0xBA..=0xC2,
    0xDB..=0xDF,
    0xE1..=0xE4,
];

/// Bits of lparam indicating scan code, including extended bit.
const SCAN_MASK: LPARAM = 0x1ff_0000;

/// Determine whether there are more messages in the queue for this key event.
///
/// When this function returns `false`, there is another message in the queue
/// with a matching scan code, therefore it is reasonable to stash the data
/// from this message and defer til later to actually produce the event.
unsafe fn is_last_message(hwnd: HWND, msg: UINT, lparam: LPARAM) -> bool {
    let expected_msg = match msg {
        WM_KEYDOWN | WM_CHAR => WM_CHAR,
        WM_SYSKEYDOWN | WM_SYSCHAR => WM_SYSCHAR,
        _ => unreachable!(),
    };
    let mut msg = mem::zeroed();
    let avail = PeekMessageW(&mut msg, hwnd, expected_msg, expected_msg, PM_NOREMOVE);
    avail == 0 || msg.lParam & SCAN_MASK != lparam & SCAN_MASK
}

const MODIFIER_MAP: &[(INT, Modifiers, SHORT)] = &[
    (VK_MENU, Modifiers::ALT, 0x80),
    (VK_CAPITAL, Modifiers::CAPS_LOCK, 0x1),
    (VK_CONTROL, Modifiers::CONTROL, 0x80),
    (VK_NUMLOCK, Modifiers::NUM_LOCK, 0x1),
    (VK_SCROLL, Modifiers::SCROLL_LOCK, 0x1),
    (VK_SHIFT, Modifiers::SHIFT, 0x80),
];

/// Convert scan code to W3C standard code.
///
/// It's hard to get an authoritative source for this; it's mostly based
/// on NativeKeyToDOMCodeName.h in Mozilla.
fn scan_to_code(scan_code: u32) -> Code {
    use Code::*;
    match scan_code {
        0x1 => Escape,
        0x2 => Digit1,
        0x3 => Digit2,
        0x4 => Digit3,
        0x5 => Digit4,
        0x6 => Digit5,
        0x7 => Digit6,
        0x8 => Digit7,
        0x9 => Digit8,
        0xA => Digit9,
        0xB => Digit0,
        0xC => Minus,
        0xD => Equal,
        0xE => Backspace,
        0xF => Tab,
        0x10 => KeyQ,
        0x11 => KeyW,
        0x12 => KeyE,
        0x13 => KeyR,
        0x14 => KeyT,
        0x15 => KeyY,
        0x16 => KeyU,
        0x17 => KeyI,
        0x18 => KeyO,
        0x19 => KeyP,
        0x1A => BracketLeft,
        0x1B => BracketRight,
        0x1C => Enter,
        0x1D => ControlLeft,
        0x1E => KeyA,
        0x1F => KeyS,
        0x20 => KeyD,
        0x21 => KeyF,
        0x22 => KeyG,
        0x23 => KeyH,
        0x24 => KeyJ,
        0x25 => KeyK,
        0x26 => KeyL,
        0x27 => Semicolon,
        0x28 => Quote,
        0x29 => Backquote,
        0x2A => ShiftLeft,
        0x2B => Backslash,
        0x2C => KeyZ,
        0x2D => KeyX,
        0x2E => KeyC,
        0x2F => KeyV,
        0x30 => KeyB,
        0x31 => KeyN,
        0x32 => KeyM,
        0x33 => Comma,
        0x34 => Period,
        0x35 => Slash,
        0x36 => ShiftRight,
        0x37 => NumpadMultiply,
        0x38 => AltLeft,
        0x39 => Space,
        0x3A => CapsLock,
        0x3B => F1,
        0x3C => F2,
        0x3D => F3,
        0x3E => F4,
        0x3F => F5,
        0x40 => F6,
        0x41 => F7,
        0x42 => F8,
        0x43 => F9,
        0x44 => F10,
        0x45 => Pause,
        0x46 => ScrollLock,
        0x47 => Numpad7,
        0x48 => Numpad8,
        0x49 => Numpad9,
        0x4A => NumpadSubtract,
        0x4B => Numpad4,
        0x4C => Numpad5,
        0x4D => Numpad6,
        0x4E => NumpadAdd,
        0x4F => Numpad1,
        0x50 => Numpad2,
        0x51 => Numpad3,
        0x52 => Numpad0,
        0x53 => NumpadDecimal,
        0x54 => PrintScreen,
        0x56 => IntlBackslash,
        0x57 => F11,
        0x58 => F12,
        0x59 => NumpadEqual,
        0x70 => KanaMode,
        0x71 => Lang2,
        0x72 => Lang1,
        0x73 => IntlRo,
        0x79 => Convert,
        0x7B => NonConvert,
        0x7D => IntlYen,
        0x7E => NumpadComma,
        0x110 => MediaTrackPrevious,
        0x119 => MediaTrackNext,
        0x11C => NumpadEnter,
        0x11D => ControlRight,
        0x120 => AudioVolumeMute,
        0x121 => LaunchApp2,
        0x122 => MediaPlayPause,
        0x124 => MediaStop,
        0x12E => AudioVolumeDown,
        0x130 => AudioVolumeUp,
        0x132 => BrowserHome,
        0x135 => NumpadDivide,
        0x137 => PrintScreen,
        0x138 => AltRight,
        0x145 => NumLock,
        0x147 => Home,
        0x148 => ArrowUp,
        0x149 => PageUp,
        0x14B => ArrowLeft,
        0x14D => ArrowRight,
        0x14F => End,
        0x150 => ArrowDown,
        0x151 => PageDown,
        0x152 => Insert,
        0x153 => Delete,
        0x15B => MetaLeft,
        0x15C => MetaRight,
        0x15D => ContextMenu,
        0x15E => Power,
        0x165 => BrowserSearch,
        0x166 => BrowserFavorites,
        0x167 => BrowserRefresh,
        0x168 => BrowserStop,
        0x169 => BrowserForward,
        0x16A => BrowserBack,
        0x16B => LaunchApp1,
        0x16C => LaunchMail,
        0x16D => MediaSelect,
        0x1F1 => Lang2,
        0x1F2 => Lang1,
        _ => Unidentified,
    }
}

fn vk_to_key(vk: VkCode) -> Option<KbKey> {
    Some(match vk as INT {
        VK_CANCEL => KbKey::Cancel,
        VK_BACK => KbKey::Backspace,
        VK_TAB => KbKey::Tab,
        VK_CLEAR => KbKey::Clear,
        VK_RETURN => KbKey::Enter,
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => KbKey::Shift,
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => KbKey::Control,
        VK_MENU | VK_LMENU | VK_RMENU => KbKey::Alt,
        VK_PAUSE => KbKey::Pause,
        VK_CAPITAL => KbKey::CapsLock,
        // TODO: disambiguate kana and hangul? same vk
        VK_KANA => KbKey::KanaMode,
        VK_JUNJA => KbKey::JunjaMode,
        VK_FINAL => KbKey::FinalMode,
        VK_KANJI => KbKey::KanjiMode,
        VK_ESCAPE => KbKey::Escape,
        VK_NONCONVERT => KbKey::NonConvert,
        VK_ACCEPT => KbKey::Accept,
        VK_PRIOR => KbKey::PageUp,
        VK_NEXT => KbKey::PageDown,
        VK_END => KbKey::End,
        VK_HOME => KbKey::Home,
        VK_LEFT => KbKey::ArrowLeft,
        VK_UP => KbKey::ArrowUp,
        VK_RIGHT => KbKey::ArrowRight,
        VK_DOWN => KbKey::ArrowDown,
        VK_SELECT => KbKey::Select,
        VK_PRINT => KbKey::Print,
        VK_EXECUTE => KbKey::Execute,
        VK_SNAPSHOT => KbKey::PrintScreen,
        VK_INSERT => KbKey::Insert,
        VK_DELETE => KbKey::Delete,
        VK_HELP => KbKey::Help,
        VK_LWIN | VK_RWIN => KbKey::Meta,
        VK_APPS => KbKey::ContextMenu,
        VK_SLEEP => KbKey::Standby,
        VK_F1 => KbKey::F1,
        VK_F2 => KbKey::F2,
        VK_F3 => KbKey::F3,
        VK_F4 => KbKey::F4,
        VK_F5 => KbKey::F5,
        VK_F6 => KbKey::F6,
        VK_F7 => KbKey::F7,
        VK_F8 => KbKey::F8,
        VK_F9 => KbKey::F9,
        VK_F10 => KbKey::F10,
        VK_F11 => KbKey::F11,
        VK_F12 => KbKey::F12,
        VK_NUMLOCK => KbKey::NumLock,
        VK_SCROLL => KbKey::ScrollLock,
        VK_BROWSER_BACK => KbKey::BrowserBack,
        VK_BROWSER_FORWARD => KbKey::BrowserForward,
        VK_BROWSER_REFRESH => KbKey::BrowserRefresh,
        VK_BROWSER_STOP => KbKey::BrowserStop,
        VK_BROWSER_SEARCH => KbKey::BrowserSearch,
        VK_BROWSER_FAVORITES => KbKey::BrowserFavorites,
        VK_BROWSER_HOME => KbKey::BrowserHome,
        VK_VOLUME_MUTE => KbKey::AudioVolumeMute,
        VK_VOLUME_DOWN => KbKey::AudioVolumeDown,
        VK_VOLUME_UP => KbKey::AudioVolumeUp,
        VK_MEDIA_NEXT_TRACK => KbKey::MediaTrackNext,
        VK_MEDIA_PREV_TRACK => KbKey::MediaTrackPrevious,
        VK_MEDIA_STOP => KbKey::MediaStop,
        VK_MEDIA_PLAY_PAUSE => KbKey::MediaPlayPause,
        VK_LAUNCH_MAIL => KbKey::LaunchMail,
        VK_LAUNCH_MEDIA_SELECT => KbKey::LaunchMediaPlayer,
        VK_LAUNCH_APP1 => KbKey::LaunchApplication1,
        VK_LAUNCH_APP2 => KbKey::LaunchApplication2,
        VK_OEM_ATTN => KbKey::Alphanumeric,
        VK_CONVERT => KbKey::Convert,
        VK_MODECHANGE => KbKey::ModeChange,
        VK_PROCESSKEY => KbKey::Process,
        VK_ATTN => KbKey::Attn,
        VK_CRSEL => KbKey::CrSel,
        VK_EXSEL => KbKey::ExSel,
        VK_EREOF => KbKey::EraseEof,
        VK_PLAY => KbKey::Play,
        VK_ZOOM => KbKey::ZoomToggle,
        VK_OEM_CLEAR => KbKey::Clear,
        _ => return None,
    })
}

/// Convert a key to a virtual key code.
///
/// The virtual key code is needed in various winapi interfaces, including
/// accelerators. This provides the virtual key code in the current keyboard
/// map.
///
/// The virtual key code can have modifiers in the higher order byte when the
/// argument is a `Character` variant. See:
/// https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-vkkeyscanw
pub(crate) fn key_to_vk(key: &KbKey) -> Option<i32> {
    Some(match key {
        KbKey::Character(s) => {
            if let Some(code_point) = s.chars().next() {
                if let Ok(wchar) = (code_point as u32).try_into() {
                    unsafe { VkKeyScanW(wchar) as i32 }
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        KbKey::Cancel => VK_CANCEL,
        KbKey::Backspace => VK_BACK,
        KbKey::Tab => VK_TAB,
        KbKey::Clear => VK_CLEAR,
        KbKey::Enter => VK_RETURN,
        KbKey::Shift => VK_SHIFT,
        KbKey::Control => VK_CONTROL,
        KbKey::Alt => VK_MENU,
        KbKey::Pause => VK_PAUSE,
        KbKey::CapsLock => VK_CAPITAL,
        // TODO: disambiguate kana and hangul? same vk
        KbKey::KanaMode => VK_KANA,
        KbKey::JunjaMode => VK_JUNJA,
        KbKey::FinalMode => VK_FINAL,
        KbKey::KanjiMode => VK_KANJI,
        KbKey::Escape => VK_ESCAPE,
        KbKey::NonConvert => VK_NONCONVERT,
        KbKey::Accept => VK_ACCEPT,
        KbKey::PageUp => VK_PRIOR,
        KbKey::PageDown => VK_NEXT,
        KbKey::End => VK_END,
        KbKey::Home => VK_HOME,
        KbKey::ArrowLeft => VK_LEFT,
        KbKey::ArrowUp => VK_UP,
        KbKey::ArrowRight => VK_RIGHT,
        KbKey::ArrowDown => VK_DOWN,
        KbKey::Select => VK_SELECT,
        KbKey::Print => VK_PRINT,
        KbKey::Execute => VK_EXECUTE,
        KbKey::PrintScreen => VK_SNAPSHOT,
        KbKey::Insert => VK_INSERT,
        KbKey::Delete => VK_DELETE,
        KbKey::Help => VK_HELP,
        KbKey::Meta => VK_LWIN,
        KbKey::ContextMenu => VK_APPS,
        KbKey::Standby => VK_SLEEP,
        KbKey::F1 => VK_F1,
        KbKey::F2 => VK_F2,
        KbKey::F3 => VK_F3,
        KbKey::F4 => VK_F4,
        KbKey::F5 => VK_F5,
        KbKey::F6 => VK_F6,
        KbKey::F7 => VK_F7,
        KbKey::F8 => VK_F8,
        KbKey::F9 => VK_F9,
        KbKey::F10 => VK_F10,
        KbKey::F11 => VK_F11,
        KbKey::F12 => VK_F12,
        KbKey::NumLock => VK_NUMLOCK,
        KbKey::ScrollLock => VK_SCROLL,
        KbKey::BrowserBack => VK_BROWSER_BACK,
        KbKey::BrowserForward => VK_BROWSER_FORWARD,
        KbKey::BrowserRefresh => VK_BROWSER_REFRESH,
        KbKey::BrowserStop => VK_BROWSER_STOP,
        KbKey::BrowserSearch => VK_BROWSER_SEARCH,
        KbKey::BrowserFavorites => VK_BROWSER_FAVORITES,
        KbKey::BrowserHome => VK_BROWSER_HOME,
        KbKey::AudioVolumeMute => VK_VOLUME_MUTE,
        KbKey::AudioVolumeDown => VK_VOLUME_DOWN,
        KbKey::AudioVolumeUp => VK_VOLUME_UP,
        KbKey::MediaTrackNext => VK_MEDIA_NEXT_TRACK,
        KbKey::MediaTrackPrevious => VK_MEDIA_PREV_TRACK,
        KbKey::MediaStop => VK_MEDIA_STOP,
        KbKey::MediaPlayPause => VK_MEDIA_PLAY_PAUSE,
        KbKey::LaunchMail => VK_LAUNCH_MAIL,
        KbKey::LaunchMediaPlayer => VK_LAUNCH_MEDIA_SELECT,
        KbKey::LaunchApplication1 => VK_LAUNCH_APP1,
        KbKey::LaunchApplication2 => VK_LAUNCH_APP2,
        KbKey::Alphanumeric => VK_OEM_ATTN,
        KbKey::Convert => VK_CONVERT,
        KbKey::ModeChange => VK_MODECHANGE,
        KbKey::Process => VK_PROCESSKEY,
        KbKey::Attn => VK_ATTN,
        KbKey::CrSel => VK_CRSEL,
        KbKey::ExSel => VK_EXSEL,
        KbKey::EraseEof => VK_EREOF,
        KbKey::Play => VK_PLAY,
        KbKey::ZoomToggle => VK_ZOOM,
        _ => return None,
    })
}

fn code_unit_to_key(code_unit: u32) -> KbKey {
    match code_unit {
        0x8 | 0x7F => KbKey::Backspace,
        0x9 => KbKey::Tab,
        0xA | 0xD => KbKey::Enter,
        0x1B => KbKey::Escape,
        _ if code_unit >= 0x20 => {
            if let Some(c) = std::char::from_u32(code_unit) {
                KbKey::Character(c.to_string())
            } else {
                // UTF-16 error, very unlikely
                KbKey::Unidentified
            }
        }
        _ => KbKey::Unidentified,
    }
}

/// Get location from virtual key code.
///
/// This logic is based on NativeKbKey::GetKeyLocation from Mozilla.
fn vk_to_location(vk: VkCode, is_extended: bool) -> Location {
    match vk as INT {
        VK_LSHIFT | VK_LCONTROL | VK_LMENU | VK_LWIN => Location::Left,
        VK_RSHIFT | VK_RCONTROL | VK_RMENU | VK_RWIN => Location::Right,
        VK_RETURN if is_extended => Location::Numpad,
        VK_INSERT | VK_DELETE | VK_END | VK_DOWN | VK_NEXT | VK_LEFT | VK_CLEAR | VK_RIGHT
        | VK_HOME | VK_UP | VK_PRIOR => {
            if is_extended {
                Location::Standard
            } else {
                Location::Numpad
            }
        }
        VK_NUMPAD0 | VK_NUMPAD1 | VK_NUMPAD2 | VK_NUMPAD3 | VK_NUMPAD4 | VK_NUMPAD5
        | VK_NUMPAD6 | VK_NUMPAD7 | VK_NUMPAD8 | VK_NUMPAD9 | VK_DECIMAL | VK_DIVIDE
        | VK_MULTIPLY | VK_SUBTRACT | VK_ADD | VK_ABNT_C2 => Location::Numpad,
        _ => Location::Standard,
    }
}

impl KeyboardState {
    /// Create a new keyboard state.
    ///
    /// There should be one of these per window. It loads the current keyboard
    /// layout and retains some mapping information from it.
    pub(crate) fn new() -> KeyboardState {
        unsafe {
            let hkl = GetKeyboardLayout(0);
            let key_vals = HashMap::new();
            let dead_keys = HashSet::new();
            let stash_vk = None;
            let stash_utf16 = Vec::new();
            let has_altgr = false;
            let mut result = KeyboardState {
                hkl,
                key_vals,
                dead_keys,
                has_altgr,
                stash_vk,
                stash_utf16,
            };
            result.load_keyboard_layout();
            result
        }
    }

    /// Process one message from the platform.
    ///
    /// This is the main interface point for generating cooked keyboard events
    /// from raw platform messages. It should be called for each relevant message,
    /// which comprises: `WM_KEYDOWN`, `WM_KEYUP`, `WM_CHAR`, `WM_SYSKEYDOWN`,
    /// `WM_SYSKEYUP`, `WM_SYSCHAR`, and `WM_INPUTLANGCHANGE`.
    ///
    /// As a general theory, many keyboard events generate a sequence of platform
    /// messages. In these cases, we stash information from all messages but the
    /// last, and generate the event from the last (using `PeekMessage` to detect
    /// that case). Mozilla handling is slightly different; it actually tries to
    /// do the processing on the first message, fetching the subsequent messages
    /// from the queue. We believe our handling is simpler and more robust.
    ///
    /// A simple example of a multi-message sequence is the key "=". In a US layout,
    /// we'd expect `WM_KEYDOWN` with `wparam = VK_OEM_PLUS` and lparam encoding the
    /// keycode that translates into `Code::Equal`, followed by a `WM_CHAR` with
    /// `wparam = b"="` and the same scancode.
    ///
    /// A more complex example of a multi-message sequence is the second press of
    /// that key in a German layout, where it's mapped to the dead key for accent
    /// acute. Then we expect `WM_KEYDOWN` with `wparam = VK_OEM_6` followed by
    /// two `WM_CHAR` with `wparam = 0xB4` (corresponding to U+00B4 = acute accent).
    /// In this case, the result (produced on the final message in the sequence) is
    /// a key event with `key = KbKey::Character("´´")`, which also matches browser
    /// behavior.
    ///
    /// # Safety
    ///
    /// The `hwnd` argument must be a valid `HWND`. Similarly, the `lparam` must be
    /// a valid `HKL` reference in the `WM_INPUTLANGCHANGE` message. Actual danger
    /// is likely low, though.
    pub(crate) unsafe fn process_message(
        &mut self,
        hwnd: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<KeyEvent> {
        match msg {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                //println!("keydown wparam {:x} lparam {:x}", wparam, lparam);
                let scan_code = ((lparam & SCAN_MASK) >> 16) as u32;
                let vk = self.refine_vk(wparam as u8, scan_code);
                if is_last_message(hwnd, msg, lparam) {
                    let mods = self.get_modifiers();
                    let code = scan_to_code(scan_code);
                    let key = vk_to_key(vk).unwrap_or_else(|| self.get_base_key(vk, mods));
                    let repeat = (lparam & 0x4000_0000) != 0;
                    let is_extended = (lparam & 0x100_0000) != 0;
                    let location = vk_to_location(vk, is_extended);
                    let state = KeyState::Down;
                    let event = KeyEvent {
                        state,
                        mods,
                        code,
                        key,
                        is_composing: false,
                        location,
                        repeat,
                    };
                    Some(event)
                } else {
                    self.stash_vk = Some(vk);
                    None
                }
            }
            WM_KEYUP | WM_SYSKEYUP => {
                let scan_code = ((lparam & SCAN_MASK) >> 16) as u32;
                let vk = self.refine_vk(wparam as u8, scan_code);
                let mods = self.get_modifiers();
                let code = scan_to_code(scan_code);
                let key = vk_to_key(vk).unwrap_or_else(|| self.get_base_key(vk, mods));
                let repeat = false;
                let is_extended = (lparam & 0x100_0000) != 0;
                let location = vk_to_location(vk, is_extended);
                let state = KeyState::Up;
                let event = KeyEvent {
                    state,
                    mods,
                    code,
                    key,
                    is_composing: false,
                    location,
                    repeat,
                };
                Some(event)
            }
            WM_CHAR | WM_SYSCHAR => {
                //println!("char wparam {:x} lparam {:x}", wparam, lparam);
                if is_last_message(hwnd, msg, lparam) {
                    let stash_vk = self.stash_vk.take();
                    let mods = self.get_modifiers();
                    let scan_code = ((lparam & SCAN_MASK) >> 16) as u32;
                    let vk = self.refine_vk(stash_vk.unwrap_or(0), scan_code);
                    let code = scan_to_code(scan_code);
                    let key = if self.stash_utf16.is_empty() && wparam < 0x20 {
                        vk_to_key(vk).unwrap_or_else(|| self.get_base_key(vk, mods))
                    } else {
                        self.stash_utf16.push(wparam as u16);
                        if let Ok(s) = String::from_utf16(&self.stash_utf16) {
                            KbKey::Character(s)
                        } else {
                            KbKey::Unidentified
                        }
                    };
                    self.stash_utf16.clear();
                    let repeat = (lparam & 0x4000_0000) != 0;
                    let is_extended = (lparam & 0x100_0000) != 0;
                    let location = vk_to_location(vk, is_extended);
                    let state = KeyState::Down;
                    let event = KeyEvent {
                        state,
                        mods,
                        code,
                        key,
                        is_composing: false,
                        location,
                        repeat,
                    };
                    Some(event)
                } else {
                    self.stash_utf16.push(wparam as u16);
                    None
                }
            }
            WM_INPUTLANGCHANGE => {
                self.hkl = lparam as HKL;
                self.load_keyboard_layout();
                None
            }
            _ => None,
        }
    }

    /// Get the modifier state.
    ///
    /// This function is designed to be called from a message handler, and
    /// gives the modifier state at the time of the message (ie is the
    /// synchronous variant). See [`GetKeyState`] for more context.
    ///
    /// The interpretation of modifiers depends on the keyboard layout, as
    /// some layouts have [AltGr] and others do not.
    ///
    /// [`GetKeyState`]: https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getkeystate
    /// [AltGr]: https://en.wikipedia.org/wiki/AltGr_key
    pub(crate) fn get_modifiers(&self) -> Modifiers {
        unsafe {
            let mut modifiers = Modifiers::empty();
            for &(vk, modifier, mask) in MODIFIER_MAP {
                if GetKeyState(vk) & mask != 0 {
                    modifiers |= modifier;
                }
            }
            if self.has_altgr && GetKeyState(VK_RMENU) & 0x80 != 0 {
                modifiers |= Modifiers::ALT_GRAPH;
                modifiers &= !(Modifiers::CONTROL | Modifiers::ALT);
            }
            modifiers
        }
    }

    /// Load a keyboard layout.
    ///
    /// We need to retain a map of virtual key codes in various modifier
    /// states, because it's not practical to query that at keyboard event
    /// time (the main culprit is that `ToUnicodeEx` is stateful).
    ///
    /// The logic is based on Mozilla KeyboardLayout::LoadLayout but is
    /// considerably simplified.
    fn load_keyboard_layout(&mut self) {
        unsafe {
            self.key_vals.clear();
            self.dead_keys.clear();
            self.has_altgr = false;
            let mut key_state = [0u8; 256];
            let mut uni_chars = [0u16; 5];
            // Right now, we're only getting the values for base and shifted
            // variants. Mozilla goes through 16 mod states.
            for shift_state in 0..N_SHIFT_STATE {
                let has_shift = shift_state & SHIFT_STATE_SHIFT != 0;
                let has_altgr = shift_state & SHIFT_STATE_ALTGR != 0;
                key_state[VK_SHIFT as usize] = if has_shift { 0x80 } else { 0 };
                key_state[VK_CONTROL as usize] = if has_altgr { 0x80 } else { 0 };
                key_state[VK_LCONTROL as usize] = if has_altgr { 0x80 } else { 0 };
                key_state[VK_MENU as usize] = if has_altgr { 0x80 } else { 0 };
                key_state[VK_RMENU as usize] = if has_altgr { 0x80 } else { 0 };
                for vk in PRINTABLE_VKS.iter().cloned().flatten() {
                    let ret = ToUnicodeEx(
                        vk as UINT,
                        0,
                        key_state.as_ptr(),
                        uni_chars.as_mut_ptr(),
                        uni_chars.len() as _,
                        0,
                        self.hkl,
                    );
                    match ret.cmp(&0) {
                        Ordering::Greater => {
                            let utf16_slice = &uni_chars[..ret as usize];
                            if let Ok(strval) = String::from_utf16(utf16_slice) {
                                self.key_vals.insert((vk, shift_state), strval);
                            }
                            // If the AltGr version of the key has a different string than
                            // the base, then the layout has AltGr. Note that Mozilla also
                            // checks dead keys for change.
                            if has_altgr
                                && !self.has_altgr
                                && self.key_vals.get(&(vk, shift_state))
                                    != self.key_vals.get(&(vk, shift_state & !SHIFT_STATE_ALTGR))
                            {
                                self.has_altgr = true;
                            }
                        }
                        Ordering::Less => {
                            // It's a dead key, press it again to reset the state.
                            self.dead_keys.insert((vk, shift_state));
                            let _ = ToUnicodeEx(
                                vk as UINT,
                                0,
                                key_state.as_ptr(),
                                uni_chars.as_mut_ptr(),
                                uni_chars.len() as _,
                                0,
                                self.hkl,
                            );
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn get_base_key(&self, vk: VkCode, modifiers: Modifiers) -> KbKey {
        let mut shift_state = 0;
        if modifiers.shift() {
            shift_state |= SHIFT_STATE_SHIFT;
        }
        if modifiers.contains(Modifiers::ALT_GRAPH) {
            shift_state |= SHIFT_STATE_ALTGR;
        }
        if let Some(s) = self.key_vals.get(&(vk, shift_state)) {
            KbKey::Character(s.clone())
        } else {
            let mapped = self.map_vk(vk);
            if mapped >= (1 << 31) {
                KbKey::Dead
            } else {
                code_unit_to_key(mapped)
            }
        }
    }

    /// Map a virtual key code to a code unit, also indicate if dead key.
    ///
    /// Bit 31 is set if the mapping is to a dead key. The bottom bits contain the code unit.
    fn map_vk(&self, vk: VkCode) -> u32 {
        unsafe { MapVirtualKeyExW(vk as _, MAPVK_VK_TO_CHAR, self.hkl) }
    }

    /// Refine a virtual key code to distinguish left and right.
    ///
    /// This only does the mapping if the original code is ambiguous, as otherwise the
    /// virtual key code reported in `wparam` is more reliable.
    fn refine_vk(&self, vk: VkCode, mut scan_code: u32) -> VkCode {
        match vk as INT {
            0 | VK_SHIFT | VK_CONTROL | VK_MENU => {
                if scan_code >= 0x100 {
                    scan_code += 0xE000 - 0x100;
                }
                unsafe { MapVirtualKeyExW(scan_code, MAPVK_VSC_TO_VK_EX, self.hkl) as u8 }
            }
            _ => vk,
        }
    }
}
