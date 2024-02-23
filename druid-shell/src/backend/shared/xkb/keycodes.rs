// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(non_upper_case_globals)]

use keyboard_types::Key;

use super::xkbcommon_sys::*;

/// Map from an xkb_common key code to a key, if possible.
pub fn map_key(keysym: u32) -> Key {
    use Key::*;
    match keysym {
        XKB_KEY_v => Key::Character("v".to_string()),
        XKB_KEY_BackSpace => Backspace,
        XKB_KEY_Tab | XKB_KEY_KP_Tab | XKB_KEY_ISO_Left_Tab => Tab,
        XKB_KEY_Clear | XKB_KEY_KP_Begin | XKB_KEY_XF86Clear => Clear,
        XKB_KEY_Return | XKB_KEY_KP_Enter => Enter,
        XKB_KEY_Linefeed => Enter,
        XKB_KEY_Pause => Pause,
        XKB_KEY_Scroll_Lock => ScrollLock,
        XKB_KEY_Escape => Escape,
        XKB_KEY_Multi_key => Compose,
        XKB_KEY_Kanji => KanjiMode,
        XKB_KEY_Muhenkan => NonConvert,
        XKB_KEY_Henkan_Mode => Convert,
        XKB_KEY_Romaji => Romaji,
        XKB_KEY_Hiragana => Hiragana,
        XKB_KEY_Katakana => Katakana,
        XKB_KEY_Hiragana_Katakana => HiraganaKatakana,
        XKB_KEY_Zenkaku => Zenkaku,
        XKB_KEY_Hankaku => Hankaku,
        XKB_KEY_Zenkaku_Hankaku => ZenkakuHankaku,
        XKB_KEY_Kana_Lock => KanaMode,
        XKB_KEY_Eisu_Shift | XKB_KEY_Eisu_toggle => Alphanumeric,
        XKB_KEY_Hangul => HangulMode,
        XKB_KEY_Hangul_Hanja => HanjaMode,
        XKB_KEY_Codeinput => CodeInput,
        XKB_KEY_SingleCandidate => SingleCandidate,
        XKB_KEY_MultipleCandidate => AllCandidates,
        XKB_KEY_PreviousCandidate => PreviousCandidate,
        XKB_KEY_Home | XKB_KEY_KP_Home => Home,
        XKB_KEY_Left | XKB_KEY_KP_Left => ArrowLeft,
        XKB_KEY_Up | XKB_KEY_KP_Up => ArrowUp,
        XKB_KEY_Right | XKB_KEY_KP_Right => ArrowRight,
        XKB_KEY_Down | XKB_KEY_KP_Down => ArrowDown,
        XKB_KEY_Prior | XKB_KEY_KP_Prior => PageUp,
        XKB_KEY_Next | XKB_KEY_KP_Next | XKB_KEY_XF86ScrollDown => PageDown,
        XKB_KEY_End | XKB_KEY_KP_End | XKB_KEY_XF86ScrollUp => End,
        XKB_KEY_Select => Select,
        // Treat Print/PrintScreen as PrintScreen https://crbug.com/683097.
        XKB_KEY_Print | XKB_KEY_3270_PrintScreen => PrintScreen,
        XKB_KEY_Execute => Execute,
        XKB_KEY_Insert | XKB_KEY_KP_Insert => Insert,
        XKB_KEY_Undo => Undo,
        XKB_KEY_Redo => Redo,
        XKB_KEY_Menu => ContextMenu,
        XKB_KEY_Find => Find,
        XKB_KEY_Cancel => Cancel,
        XKB_KEY_Help => Help,
        XKB_KEY_Break | XKB_KEY_3270_Attn => Attn,
        XKB_KEY_Mode_switch => ModeChange,
        XKB_KEY_Num_Lock => NumLock,
        XKB_KEY_F1 | XKB_KEY_KP_F1 => F1,
        XKB_KEY_F2 | XKB_KEY_KP_F2 => F2,
        XKB_KEY_F3 | XKB_KEY_KP_F3 => F3,
        XKB_KEY_F4 | XKB_KEY_KP_F4 => F4,
        XKB_KEY_F5 => F5,
        XKB_KEY_F6 => F6,
        XKB_KEY_F7 => F7,
        XKB_KEY_F8 => F8,
        XKB_KEY_F9 => F9,
        XKB_KEY_F10 => F10,
        XKB_KEY_F11 => F11,
        XKB_KEY_F12 => F12,
        XKB_KEY_XF86Tools | XKB_KEY_F13 => F13,
        XKB_KEY_F14 | XKB_KEY_XF86Launch5 => F14,
        XKB_KEY_F15 | XKB_KEY_XF86Launch6 => F15,
        XKB_KEY_F16 | XKB_KEY_XF86Launch7 => F16,
        XKB_KEY_F17 | XKB_KEY_XF86Launch8 => F17,
        XKB_KEY_F18 | XKB_KEY_XF86Launch9 => F18,
        XKB_KEY_F19 => F19,
        XKB_KEY_F20 => F20,
        XKB_KEY_F21 => F21,
        XKB_KEY_F22 => F22,
        XKB_KEY_F23 => F23,
        XKB_KEY_F24 => F24,
        // not available in keyboard-types
        // XKB_KEY_XF86Calculator => LaunchCalculator,
        // XKB_KEY_XF86MyComputer | XKB_KEY_XF86Explorer => LaunchMyComputer,
        // XKB_KEY_ISO_Level3_Latch => AltGraphLatch,
        // XKB_KEY_ISO_Level5_Shift => ShiftLevel5,
        XKB_KEY_Shift_L | XKB_KEY_Shift_R => Shift,
        XKB_KEY_Control_L | XKB_KEY_Control_R => Control,
        XKB_KEY_Caps_Lock => CapsLock,
        XKB_KEY_Meta_L | XKB_KEY_Meta_R => Meta,
        XKB_KEY_Alt_L | XKB_KEY_Alt_R => Alt,
        XKB_KEY_Super_L | XKB_KEY_Super_R => Meta,
        XKB_KEY_Hyper_L | XKB_KEY_Hyper_R => Hyper,
        XKB_KEY_Delete => Delete,
        XKB_KEY_SunProps => Props,
        XKB_KEY_XF86Next_VMode => VideoModeNext,
        XKB_KEY_XF86MonBrightnessUp => BrightnessUp,
        XKB_KEY_XF86MonBrightnessDown => BrightnessDown,
        XKB_KEY_XF86Standby | XKB_KEY_XF86Sleep | XKB_KEY_XF86Suspend => Standby,
        XKB_KEY_XF86AudioLowerVolume => AudioVolumeDown,
        XKB_KEY_XF86AudioMute => AudioVolumeMute,
        XKB_KEY_XF86AudioRaiseVolume => AudioVolumeUp,
        XKB_KEY_XF86AudioPlay => MediaPlayPause,
        XKB_KEY_XF86AudioStop => MediaStop,
        XKB_KEY_XF86AudioPrev => MediaTrackPrevious,
        XKB_KEY_XF86AudioNext => MediaTrackNext,
        XKB_KEY_XF86HomePage => BrowserHome,
        XKB_KEY_XF86Mail => LaunchMail,
        XKB_KEY_XF86Search => BrowserSearch,
        XKB_KEY_XF86AudioRecord => MediaRecord,
        XKB_KEY_XF86Calendar => LaunchCalendar,
        XKB_KEY_XF86Back => BrowserBack,
        XKB_KEY_XF86Forward => BrowserForward,
        XKB_KEY_XF86Stop => BrowserStop,
        XKB_KEY_XF86Refresh | XKB_KEY_XF86Reload => BrowserRefresh,
        XKB_KEY_XF86PowerOff => PowerOff,
        XKB_KEY_XF86WakeUp => WakeUp,
        XKB_KEY_XF86Eject => Eject,
        XKB_KEY_XF86ScreenSaver => LaunchScreenSaver,
        XKB_KEY_XF86WWW => LaunchWebBrowser,
        XKB_KEY_XF86Favorites => BrowserFavorites,
        XKB_KEY_XF86AudioPause => MediaPause,
        XKB_KEY_XF86AudioMedia | XKB_KEY_XF86Music => LaunchMusicPlayer,
        XKB_KEY_XF86AudioRewind => MediaRewind,
        XKB_KEY_XF86CD | XKB_KEY_XF86Video => LaunchMediaPlayer,
        XKB_KEY_XF86Close => Close,
        XKB_KEY_XF86Copy | XKB_KEY_SunCopy => Copy,
        XKB_KEY_XF86Cut | XKB_KEY_SunCut => Cut,
        XKB_KEY_XF86Display => DisplaySwap,
        XKB_KEY_XF86Excel => LaunchSpreadsheet,
        XKB_KEY_XF86LogOff => LogOff,
        XKB_KEY_XF86New => New,
        XKB_KEY_XF86Open | XKB_KEY_SunOpen => Open,
        XKB_KEY_XF86Paste | XKB_KEY_SunPaste => Paste,
        XKB_KEY_XF86Reply => MailReply,
        XKB_KEY_XF86Save => Save,
        XKB_KEY_XF86Send => MailSend,
        XKB_KEY_XF86Spell => SpellCheck,
        XKB_KEY_XF86SplitScreen => SplitScreenToggle,
        XKB_KEY_XF86Word | XKB_KEY_XF86OfficeHome => LaunchWordProcessor,
        XKB_KEY_XF86ZoomIn => ZoomIn,
        XKB_KEY_XF86ZoomOut => ZoomOut,
        XKB_KEY_XF86WebCam => LaunchWebCam,
        XKB_KEY_XF86MailForward => MailForward,
        XKB_KEY_XF86AudioForward => MediaFastForward,
        XKB_KEY_XF86AudioRandomPlay => RandomToggle,
        XKB_KEY_XF86Subtitle => Subtitle,
        XKB_KEY_XF86Hibernate => Hibernate,
        XKB_KEY_3270_EraseEOF => EraseEof,
        XKB_KEY_3270_Play => Play,
        XKB_KEY_3270_ExSelect => ExSel,
        XKB_KEY_3270_CursorSelect => CrSel,
        XKB_KEY_ISO_Level3_Shift => AltGraph,
        XKB_KEY_ISO_Next_Group => GroupNext,
        XKB_KEY_ISO_Prev_Group => GroupPrevious,
        XKB_KEY_ISO_First_Group => GroupFirst,
        XKB_KEY_ISO_Last_Group => GroupLast,
        XKB_KEY_dead_grave
        | XKB_KEY_dead_acute
        | XKB_KEY_dead_circumflex
        | XKB_KEY_dead_tilde
        | XKB_KEY_dead_macron
        | XKB_KEY_dead_breve
        | XKB_KEY_dead_abovedot
        | XKB_KEY_dead_diaeresis
        | XKB_KEY_dead_abovering
        | XKB_KEY_dead_doubleacute
        | XKB_KEY_dead_caron
        | XKB_KEY_dead_cedilla
        | XKB_KEY_dead_ogonek
        | XKB_KEY_dead_iota
        | XKB_KEY_dead_voiced_sound
        | XKB_KEY_dead_semivoiced_sound
        | XKB_KEY_dead_belowdot
        | XKB_KEY_dead_hook
        | XKB_KEY_dead_horn
        | XKB_KEY_dead_stroke
        | XKB_KEY_dead_abovecomma
        | XKB_KEY_dead_abovereversedcomma
        | XKB_KEY_dead_doublegrave
        | XKB_KEY_dead_belowring
        | XKB_KEY_dead_belowmacron
        | XKB_KEY_dead_belowcircumflex
        | XKB_KEY_dead_belowtilde
        | XKB_KEY_dead_belowbreve
        | XKB_KEY_dead_belowdiaeresis
        | XKB_KEY_dead_invertedbreve
        | XKB_KEY_dead_belowcomma
        | XKB_KEY_dead_currency
        | XKB_KEY_dead_greek => Dead,
        _ => Unidentified,
    }
}
