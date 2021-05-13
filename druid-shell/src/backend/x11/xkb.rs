// Copyright 2021 The Druid Authors.
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

//! A minimal wrapper around Xkb for our use.

#![allow(non_upper_case_globals)]

use super::xkbcommon_sys::*;
use crate::{
    backend::shared::{code_to_location, hardware_keycode_to_code},
    KeyEvent, KeyState, Modifiers,
};
use keyboard_types::{Code, Key};
use std::os::raw::c_int;
use std::{convert::TryFrom, ptr};
use x11rb::xcb_ffi::XCBConnection;

pub struct DeviceId(c_int);

/// A global xkb context object.
///
/// Reference counted under the hood.
// Assume this isn't threadsafe unless proved otherwise. (e.g. don't implement Send/Sync)
// TODO do we need UnsafeCell?
pub struct Context(*mut xkb_context);

impl Context {
    /// Create a new xkb context.
    ///
    /// The returned object is lightweight and clones will point at the same context internally.
    pub fn new() -> Self {
        unsafe { Self(xkb_context_new(XKB_CONTEXT_NO_FLAGS)) }
    }

    pub fn core_keyboard_device_id(&self, conn: &XCBConnection) -> Option<DeviceId> {
        let id = unsafe {
            xkb_x11_get_core_keyboard_device_id(
                conn.get_raw_xcb_connection() as *mut xcb_connection_t
            )
        };
        if id != -1 {
            Some(DeviceId(id))
        } else {
            None
        }
    }

    pub fn keymap_from_device(&self, conn: &XCBConnection, device: DeviceId) -> Option<Keymap> {
        let key_map = unsafe {
            xkb_x11_keymap_new_from_device(
                self.0,
                conn.get_raw_xcb_connection() as *mut xcb_connection_t,
                device.0,
                XKB_KEYMAP_COMPILE_NO_FLAGS,
            )
        };
        if key_map.is_null() {
            return None;
        }
        Some(Keymap(key_map))
    }

    /// Set the log level using `log` levels.
    ///
    /// Becuase `xkb` has a `critical` error, each rust error maps to 1 above (e.g. error ->
    /// critical, warn -> error etc.)
    pub fn set_log_level(&self, level: tracing::Level) {
        use tracing::Level;
        let level = match level {
            Level::ERROR => XKB_LOG_LEVEL_CRITICAL,
            Level::WARN => XKB_LOG_LEVEL_ERROR,
            Level::INFO => XKB_LOG_LEVEL_WARNING,
            Level::DEBUG => XKB_LOG_LEVEL_INFO,
            Level::TRACE => XKB_LOG_LEVEL_DEBUG,
        };
        unsafe {
            xkb_context_set_log_level(self.0, level);
        }
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self(unsafe { xkb_context_ref(self.0) })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            xkb_context_unref(self.0);
        }
    }
}

pub struct Keymap(*mut xkb_keymap);

impl Keymap {
    pub fn state(&self) -> State {
        State(unsafe { xkb_state_new(self.0) })
    }
}

impl Clone for Keymap {
    fn clone(&self) -> Self {
        Self(unsafe { xkb_keymap_ref(self.0) })
    }
}

impl Drop for Keymap {
    fn drop(&mut self) {
        unsafe {
            xkb_keymap_unref(self.0);
        }
    }
}

pub struct State(*mut xkb_state);

impl State {
    pub fn key_event(&self, scancode: u32, state: KeyState) -> KeyEvent {
        let code = u16::try_from(scancode)
            .map(hardware_keycode_to_code)
            .unwrap_or(Code::Unidentified);
        let key = self.get_logical_key(scancode);
        // TODO this is lazy - really should use xkb i.e. augment the get_logical_key method.
        let location = code_to_location(code);

        let repeat = false;
        // TODO not sure how to get this
        let is_composing = false;

        let mut mods = Modifiers::empty();
        // Update xkb's state (e.g. return capitals if we've pressed shift)
        unsafe {
            xkb_state_update_key(
                self.0,
                scancode,
                match state {
                    KeyState::Down => XKB_KEY_DOWN,
                    KeyState::Up => XKB_KEY_UP,
                },
            );
            if xkb_state_mod_name_is_active(
                self.0,
                XKB_MOD_NAME_CTRL.as_ptr() as *const _,
                XKB_STATE_MODS_EFFECTIVE,
            ) != 0
            {
                mods |= Modifiers::CONTROL;
            }
            if xkb_state_mod_name_is_active(
                self.0,
                XKB_MOD_NAME_SHIFT.as_ptr() as *const _,
                XKB_STATE_MODS_EFFECTIVE,
            ) != 0
            {
                mods |= Modifiers::SHIFT;
            }
            if xkb_state_mod_name_is_active(
                self.0,
                XKB_MOD_NAME_ALT.as_ptr() as *const _,
                XKB_STATE_MODS_EFFECTIVE,
            ) != 0
            {
                mods |= Modifiers::ALT;
            }
            if xkb_state_mod_name_is_active(
                self.0,
                XKB_MOD_NAME_CAPS.as_ptr() as *const _,
                XKB_STATE_MODS_EFFECTIVE,
            ) != 0
            {
                mods |= Modifiers::CAPS_LOCK;
            }
            if xkb_state_mod_name_is_active(
                self.0,
                XKB_MOD_NAME_LOGO.as_ptr() as *const _,
                XKB_STATE_MODS_EFFECTIVE,
            ) != 0
            {
                mods |= Modifiers::SUPER;
            }
            if xkb_state_mod_name_is_active(
                self.0,
                XKB_MOD_NAME_NUM.as_ptr() as *const _,
                XKB_STATE_MODS_EFFECTIVE,
            ) != 0
            {
                mods |= Modifiers::NUM_LOCK;
            }
        }
        KeyEvent {
            state,
            key,
            code,
            location,
            mods,
            repeat,
            is_composing,
        }
    }

    fn get_logical_key(&self, scancode: u32) -> Key {
        let mut key = map_key(self.key_get_one_sym(scancode));
        if matches!(key, Key::Unidentified) {
            if let Some(s) = self.key_get_utf8(scancode) {
                key = Key::Character(s);
            }
        }
        key
    }

    fn key_get_one_sym(&self, scancode: u32) -> u32 {
        unsafe { xkb_state_key_get_one_sym(self.0, scancode) }
    }

    /// Get the string representation of a key.
    // TODO `keyboard_types` forces us to return a String, but it would be nicer if we could stay
    // on the stack, especially since we expect most results to be pretty small.
    fn key_get_utf8(&self, scancode: u32) -> Option<String> {
        unsafe {
            // First get the size we will need
            let len = xkb_state_key_get_utf8(self.0, scancode, ptr::null_mut(), 0);
            if len == 0 {
                return None;
            }
            // add 1 because we will get a null-terminated string.
            let len = usize::try_from(len).unwrap() + 1;
            let mut buf: Vec<u8> = Vec::new();
            buf.resize(len, 0);
            xkb_state_key_get_utf8(self.0, scancode, buf.as_mut_ptr() as *mut i8, len);
            debug_assert!(buf[buf.len() - 1] == 0);
            buf.pop();
            Some(String::from_utf8(buf).unwrap())
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self(unsafe { xkb_state_ref(self.0) })
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            xkb_state_unref(self.0);
        }
    }
}

/// Map from an xkb_common key code to a key, if possible.
// I couldn't find the commented out keys
fn map_key(keysym: u32) -> Key {
    use Key::*;
    match keysym {
        XKB_KEY_Alt_L => Alt,
        XKB_KEY_Alt_R => AltGraph,
        XKB_KEY_Caps_Lock => CapsLock,
        XKB_KEY_Control_L | XKB_KEY_Control_R => Control,
        XKB_KEY_function => Fn,
        // FnLock, - can't find in xkb
        XKB_KEY_Meta_L | XKB_KEY_Meta_R => Meta,
        XKB_KEY_Num_Lock => NumLock,
        XKB_KEY_Scroll_Lock => ScrollLock,
        XKB_KEY_Shift_L | XKB_KEY_Shift_R => Shift,
        // Symbol,
        // SymbolLock,
        XKB_KEY_Hyper_L | XKB_KEY_Hyper_R => Hyper,
        XKB_KEY_Super_L | XKB_KEY_Super_R => Super,
        XKB_KEY_Return => Enter,
        XKB_KEY_Tab => Tab,
        XKB_KEY_Down => ArrowDown,
        XKB_KEY_Left => ArrowLeft,
        XKB_KEY_Right => ArrowRight,
        XKB_KEY_Up => ArrowUp,
        XKB_KEY_End => End,
        XKB_KEY_Home => Home,
        XKB_KEY_Page_Down => PageDown,
        XKB_KEY_Page_Up => PageUp,
        XKB_KEY_BackSpace => Backspace,
        XKB_KEY_Clear => Clear,
        XKB_KEY_XF86Copy => Copy,
        XKB_KEY_3270_CursorSelect => CrSel,
        XKB_KEY_Delete => Delete,
        XKB_KEY_Insert => Insert,
        XKB_KEY_Redo => Redo,
        XKB_KEY_Undo => Undo,
        XKB_KEY_3270_Attn => Attn,
        XKB_KEY_Cancel => Cancel,
        XKB_KEY_Menu => ContextMenu,
        XKB_KEY_Escape => Escape,
        XKB_KEY_Execute => Execute,
        XKB_KEY_Find => Find,
        XKB_KEY_Help => Help,
        XKB_KEY_Pause => Pause,
        XKB_KEY_3270_Play => Play,
        XKB_KEY_SunProps => Props,
        XKB_KEY_Select => Select,
        XKB_KEY_XF86ZoomIn => ZoomIn,
        XKB_KEY_XF86ZoomOut => ZoomOut,
        XKB_KEY_XF86MonBrightnessDown => BrightnessDown,
        XKB_KEY_XF86MonBrightnessUp => BrightnessUp,
        XKB_KEY_F1 => F1,
        XKB_KEY_F2 => F2,
        XKB_KEY_F3 => F3,
        XKB_KEY_F4 => F4,
        XKB_KEY_F5 => F5,
        XKB_KEY_F6 => F6,
        XKB_KEY_F7 => F7,
        XKB_KEY_F8 => F8,
        XKB_KEY_F9 => F9,
        XKB_KEY_F10 => F10,
        XKB_KEY_F11 => F11,
        XKB_KEY_F12 => F12,
        /* todo carry on
        Again,
        Accept,
        EraseEof,
        Paste,
        ExSel,
        Eject,
        LogOff,
        Power,
        PowerOff,
        PrintScreen,
        Hibernate,
        Standby,
        WakeUp,
        AllCandidates,
        Alphanumeric,
        CodeInput,
        Compose,
        Convert,
        Dead,
        FinalMode,
        GroupFirst,
        GroupLast,
        GroupNext,
        GroupPrevious,
        ModeChange,
        NextCandidate,
        NonConvert,
        PreviousCandidate,
        Process,
        SingleCandidate,
        HangulMode,
        HanjaMode,
        JunjaMode,
        Eisu,
        Hankaku,
        Hiragana,
        HiraganaKatakana,
        KanaMode,
        KanjiMode,
        Katakana,
        Romaji,
        Zenkaku,
        ZenkakuHankaku,
        Soft1,
        Soft2,
        Soft3,
        Soft4,
        ChannelDown,
        ChannelUp,
        Close,
        MailForward,
        MailReply,
        MailSend,
        MediaClose,
        MediaFastForward,
        MediaPause,
        MediaPlay,
        MediaPlayPause,
        MediaRecord,
        MediaRewind,
        MediaStop,
        MediaTrackNext,
        MediaTrackPrevious,
        New,
        Open,
        Print,
        Save,
        SpellCheck,
        Key11,
        Key12,
        AudioBalanceLeft,
        AudioBalanceRight,
        AudioBassBoostDown,
        AudioBassBoostToggle,
        AudioBassBoostUp,
        AudioFaderFront,
        AudioFaderRear,
        AudioSurroundModeNext,
        AudioTrebleDown,
        AudioTrebleUp,
        AudioVolumeDown,
        AudioVolumeUp,
        AudioVolumeMute,
        MicrophoneToggle,
        MicrophoneVolumeDown,
        MicrophoneVolumeUp,
        MicrophoneVolumeMute,
        SpeechCorrectionList,
        SpeechInputToggle,
        LaunchApplication1,
        LaunchApplication2,
        LaunchCalendar,
        LaunchContacts,
        LaunchMail,
        LaunchMediaPlayer,
        LaunchMusicPlayer,
        LaunchPhone,
        LaunchScreenSaver,
        LaunchSpreadsheet,
        LaunchWebBrowser,
        LaunchWebCam,
        LaunchWordProcessor,
        BrowserBack,
        BrowserFavorites,
        BrowserForward,
        BrowserHome,
        BrowserRefresh,
        BrowserSearch,
        BrowserStop,
        AppSwitch,
        Call,
        Camera,
        CameraFocus,
        EndCall,
        GoBack,
        GoHome,
        HeadsetHook,
        LastNumberRedial,
        Notification,
        MannerMode,
        VoiceDial,
        TV,
        TV3DMode,
        TVAntennaCable,
        TVAudioDescription,
        TVAudioDescriptionMixDown,
        TVAudioDescriptionMixUp,
        TVContentsMenu,
        TVDataService,
        TVInput,
        TVInputComponent1,
        TVInputComponent2,
        TVInputComposite1,
        TVInputComposite2,
        TVInputHDMI1,
        TVInputHDMI2,
        TVInputHDMI3,
        TVInputHDMI4,
        TVInputVGA1,
        TVMediaContext,
        TVNetwork,
        TVNumberEntry,
        TVPower,
        TVRadioService,
        TVSatellite,
        TVSatelliteBS,
        TVSatelliteCS,
        TVSatelliteToggle,
        TVTerrestrialAnalog,
        TVTerrestrialDigital,
        TVTimer,
        AVRInput,
        AVRPower,
        ColorF0Red,
        ColorF1Green,
        ColorF2Yellow,
        ColorF3Blue,
        ColorF4Grey,
        ColorF5Brown,
        ClosedCaptionToggle,
        Dimmer,
        DisplaySwap,
        DVR,
        Exit,
        FavoriteClear0,
        FavoriteClear1,
        FavoriteClear2,
        FavoriteClear3,
        FavoriteRecall0,
        FavoriteRecall1,
        FavoriteRecall2,
        FavoriteRecall3,
        FavoriteStore0,
        FavoriteStore1,
        FavoriteStore2,
        FavoriteStore3,
        Guide,
        GuideNextDay,
        GuidePreviousDay,
        Info,
        InstantReplay,
        Link,
        ListProgram,
        LiveContent,
        Lock,
        MediaApps,
        MediaAudioTrack,
        MediaLast,
        MediaSkipBackward,
        MediaSkipForward,
        MediaStepBackward,
        MediaStepForward,
        MediaTopMenu,
        NavigateIn,
        NavigateNext,
        NavigateOut,
        NavigatePrevious,
        NextFavoriteChannel,
        NextUserProfile,
        OnDemand,
        Pairing,
        PinPDown,
        PinPMove,
        PinPToggle,
        PinPUp,
        PlaySpeedDown,
        PlaySpeedReset,
        PlaySpeedUp,
        RandomToggle,
        RcLowBattery,
        RecordSpeedNext,
        RfBypass,
        ScanChannelsToggle,
        ScreenModeNext,
        Settings,
        SplitScreenToggle,
        STBInput,
        STBPower,
        Subtitle,
        Teletext,
        VideoModeNext,
        Wink,
        ZoomToggle,
        */
        // A catchall
        _ => Unidentified,
    }
}
