// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! GTK code handling.

use gtk::gdk::keys::constants::*;

pub use super::super::shared::hardware_keycode_to_code;
use crate::keyboard_types::{Key, Location};

pub type RawKey = gtk::gdk::keys::Key;

#[allow(clippy::just_underscores_and_digits, non_upper_case_globals)]
pub fn raw_key_to_key(raw: RawKey) -> Option<Key> {
    // changes from x11 backend keycodes:
    // * XKB_KEY_ prefix removed
    // * 3270 is replaced with _3270
    // * XF86 prefix is removed
    // * Sun* Keys are gone
    Some(match raw {
        BackSpace => Key::Backspace,
        Tab | KP_Tab | ISO_Left_Tab => Key::Tab,
        Clear | KP_Begin => Key::Clear,
        Return | KP_Enter => Key::Enter,
        Linefeed => Key::Enter,
        Pause => Key::Pause,
        Scroll_Lock => Key::ScrollLock,
        Escape => Key::Escape,
        Multi_key => Key::Compose,
        Kanji => Key::KanjiMode,
        Muhenkan => Key::NonConvert,
        Henkan_Mode => Key::Convert,
        Romaji => Key::Romaji,
        Hiragana => Key::Hiragana,
        Katakana => Key::Katakana,
        Hiragana_Katakana => Key::HiraganaKatakana,
        Zenkaku => Key::Zenkaku,
        Hankaku => Key::Hankaku,
        Zenkaku_Hankaku => Key::ZenkakuHankaku,
        Kana_Lock => Key::KanaMode,
        Eisu_Shift | Eisu_toggle => Key::Alphanumeric,
        Hangul => Key::HangulMode,
        Hangul_Hanja => Key::HanjaMode,
        Codeinput => Key::CodeInput,
        SingleCandidate => Key::SingleCandidate,
        MultipleCandidate => Key::AllCandidates,
        PreviousCandidate => Key::PreviousCandidate,
        Home | KP_Home => Key::Home,
        Left | KP_Left => Key::ArrowLeft,
        Up | KP_Up => Key::ArrowUp,
        Right | KP_Right => Key::ArrowRight,
        Down | KP_Down => Key::ArrowDown,
        Prior | KP_Prior => Key::PageUp,
        Next | KP_Next => Key::PageDown,
        End | KP_End => Key::End,
        Select => Key::Select,
        // Treat Print/PrintScreen as PrintScreen https://crbug.com/683097.
        Print | _3270_PrintScreen => Key::PrintScreen,
        Execute => Key::Execute,
        Insert | KP_Insert => Key::Insert,
        Undo => Key::Undo,
        Redo => Key::Redo,
        Menu => Key::ContextMenu,
        Find => Key::Find,
        Cancel => Key::Cancel,
        Help => Key::Help,
        Break | _3270_Attn => Key::Attn,
        Mode_switch => Key::ModeChange,
        Num_Lock => Key::NumLock,
        F1 | KP_F1 => Key::F1,
        F2 | KP_F2 => Key::F2,
        F3 | KP_F3 => Key::F3,
        F4 | KP_F4 => Key::F4,
        F5 => Key::F5,
        F6 => Key::F6,
        F7 => Key::F7,
        F8 => Key::F8,
        F9 => Key::F9,
        F10 => Key::F10,
        F11 => Key::F11,
        F12 => Key::F12,
        Tools | F13 => Key::F13,
        F14 | Launch5 => Key::F14,
        F15 | Launch6 => Key::F15,
        F16 | Launch7 => Key::F16,
        F17 | Launch8 => Key::F17,
        F18 | Launch9 => Key::F18,
        F19 => Key::F19,
        F20 => Key::F20,
        F21 => Key::F21,
        F22 => Key::F22,
        F23 => Key::F23,
        F24 => Key::F24,
        // not available in keyboard-types
        // Calculator => Key::LaunchCalculator,
        // MyComputer | Explorer => Key::LaunchMyComputer,
        // ISO_Level3_Latch => Key::AltGraphLatch,
        // ISO_Level5_Shift => Key::ShiftLevel5,
        Shift_L | Shift_R => Key::Shift,
        Control_L | Control_R => Key::Control,
        Caps_Lock => Key::CapsLock,
        Meta_L | Meta_R => Key::Meta,
        Alt_L | Alt_R => Key::Alt,
        Super_L | Super_R => Key::Meta,
        Hyper_L | Hyper_R => Key::Hyper,
        Delete => Key::Delete,
        Next_VMode => Key::VideoModeNext,
        MonBrightnessUp => Key::BrightnessUp,
        MonBrightnessDown => Key::BrightnessDown,
        Standby | Sleep | Suspend => Key::Standby,
        AudioLowerVolume => Key::AudioVolumeDown,
        AudioMute => Key::AudioVolumeMute,
        AudioRaiseVolume => Key::AudioVolumeUp,
        AudioPlay => Key::MediaPlayPause,
        AudioStop => Key::MediaStop,
        AudioPrev => Key::MediaTrackPrevious,
        AudioNext => Key::MediaTrackNext,
        HomePage => Key::BrowserHome,
        Mail => Key::LaunchMail,
        Search => Key::BrowserSearch,
        AudioRecord => Key::MediaRecord,
        Calendar => Key::LaunchCalendar,
        Back => Key::BrowserBack,
        Forward => Key::BrowserForward,
        Stop => Key::BrowserStop,
        Refresh | Reload => Key::BrowserRefresh,
        PowerOff => Key::PowerOff,
        WakeUp => Key::WakeUp,
        Eject => Key::Eject,
        ScreenSaver => Key::LaunchScreenSaver,
        WWW => Key::LaunchWebBrowser,
        Favorites => Key::BrowserFavorites,
        AudioPause => Key::MediaPause,
        AudioMedia | Music => Key::LaunchMusicPlayer,
        AudioRewind => Key::MediaRewind,
        CD | Video => Key::LaunchMediaPlayer,
        Close => Key::Close,
        Copy => Key::Copy,
        Cut => Key::Cut,
        Display => Key::DisplaySwap,
        Excel => Key::LaunchSpreadsheet,
        LogOff => Key::LogOff,
        New => Key::New,
        Open => Key::Open,
        Paste => Key::Paste,
        Reply => Key::MailReply,
        Save => Key::Save,
        Send => Key::MailSend,
        Spell => Key::SpellCheck,
        SplitScreen => Key::SplitScreenToggle,
        Word | OfficeHome => Key::LaunchWordProcessor,
        ZoomIn => Key::ZoomIn,
        ZoomOut => Key::ZoomOut,
        WebCam => Key::LaunchWebCam,
        MailForward => Key::MailForward,
        AudioForward => Key::MediaFastForward,
        AudioRandomPlay => Key::RandomToggle,
        Subtitle => Key::Subtitle,
        Hibernate => Key::Hibernate,
        _3270_EraseEOF => Key::EraseEof,
        _3270_Play => Key::Play,
        _3270_ExSelect => Key::ExSel,
        _3270_CursorSelect => Key::CrSel,
        ISO_Level3_Shift => Key::AltGraph,
        ISO_Next_Group => Key::GroupNext,
        ISO_Prev_Group => Key::GroupPrevious,
        ISO_First_Group => Key::GroupFirst,
        ISO_Last_Group => Key::GroupLast,
        dead_grave
        | dead_acute
        | dead_circumflex
        | dead_tilde
        | dead_macron
        | dead_breve
        | dead_abovedot
        | dead_diaeresis
        | dead_abovering
        | dead_doubleacute
        | dead_caron
        | dead_cedilla
        | dead_ogonek
        | dead_iota
        | dead_voiced_sound
        | dead_semivoiced_sound
        | dead_belowdot
        | dead_hook
        | dead_horn
        | dead_stroke
        | dead_abovecomma
        | dead_abovereversedcomma
        | dead_doublegrave
        | dead_belowring
        | dead_belowmacron
        | dead_belowcircumflex
        | dead_belowtilde
        | dead_belowbreve
        | dead_belowdiaeresis
        | dead_invertedbreve
        | dead_belowcomma
        | dead_currency
        | dead_greek => Key::Dead,
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
        Key::F13 => F13,
        Key::F14 => F14,
        Key::F15 => F15,
        Key::F16 => F16,
        Key::F17 => F17,
        Key::F18 => F18,
        Key::F19 => F19,
        Key::F20 => F20,
        Key::F21 => F21,
        Key::F22 => F22,
        Key::F23 => F23,
        Key::F24 => F24,

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
