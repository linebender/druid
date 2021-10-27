//! A minimal wrapper around Xkb for our use.

#![allow(non_upper_case_globals)]

use crate::{
    backend::shared::{code_to_location, hardware_keycode_to_code},
    KeyEvent, KeyState, Modifiers,
};
use keyboard_types::{Code, Key};
use std::{convert::TryFrom, ptr};
use wayland_client::protocol::wl_keyboard as wlkeyboard;
use xkbcommon_sys::*;

struct ModMap(u32, Modifiers);

impl ModMap {
    fn merge(self, m: Modifiers, mods: u32, locked: u32) -> Modifiers {
        if self.0 & mods == 0 && self.0 & locked == 0 {
            return m;
        }

        return m | self.1;
    }
}

const MOD_SHIFT: ModMap = ModMap(1, Modifiers::SHIFT);
const MOD_CAP_LOCK: ModMap = ModMap(2, Modifiers::CAPS_LOCK);
const MOD_CTRL: ModMap = ModMap(4, Modifiers::CONTROL);
const MOD_ALT: ModMap = ModMap(8, Modifiers::ALT);
const MOD_NUM_LOCK: ModMap = ModMap(16, Modifiers::NUM_LOCK);
const MOD_META: ModMap = ModMap(64, Modifiers::META);

pub fn event_to_mods(event: wlkeyboard::Event) -> Modifiers {
    match event {
        wlkeyboard::Event::Modifiers {
            mods_depressed,
            mods_locked,
            ..
        } => {
            let mods = Modifiers::empty();
            let mods = MOD_SHIFT.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_CAP_LOCK.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_CTRL.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_ALT.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_NUM_LOCK.merge(mods, mods_depressed, mods_locked);
            let mods = MOD_META.merge(mods, mods_depressed, mods_locked);
            return mods;
        }
        _ => return Modifiers::empty(),
    }
}

//const MAX_KEY_LEN: usize = 32;

/// A global xkb context object.
///
/// Reference counted under the hood.
// Assume this isn't threadsafe unless proved otherwise. (e.g. don't implement Send/Sync)
// TODO do we need UnsafeCell?
pub struct Context {
    inner: *mut xkb_context,
}

impl Context {
    /// Create a new xkb context.
    ///
    /// The returned object is lightweight and clones will point at the same context internally.
    pub fn new() -> Self {
        unsafe {
            let inner = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
            Context { inner }
        }
    }

    /// Create a keymap from some given data.
    ///
    /// Uses `xkb_keymap_new_from_buffer` under the hood.
    pub fn keymap_from_slice(&self, buffer: &[u8]) -> Keymap {
        // TODO we hope that the keymap doesn't borrow the underlying data. If it does' we need to
        // use Rc. We'll find out soon enough if we get a segfault.
        // TODO we hope that the keymap inc's the reference count of the context.
        assert!(
            buffer.iter().copied().any(|byte| byte == 0),
            "`keymap_from_slice` expects a null-terminated string"
        );
        unsafe {
            let keymap = xkb_keymap_new_from_string(
                self.inner,
                buffer.as_ptr() as *const i8,
                XKB_KEYMAP_FORMAT_TEXT_V1,
                XKB_KEYMAP_COMPILE_NO_FLAGS,
            );
            assert!(!keymap.is_null());
            Keymap::new(keymap)
        }
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        unsafe {
            xkb_context_ref(self.inner);
            Context { inner: self.inner }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            xkb_context_unref(self.inner);
        }
    }
}

pub struct Keymap {
    inner: *mut xkb_keymap,
}

impl Keymap {
    /// Create a new keymap object.
    ///
    /// # Safety
    ///
    /// This function takes ownership of the `xkb_keymap`. It must be valid when this function is
    /// called, and not deallocated elsewhere.
    unsafe fn new(inner: *mut xkb_keymap) -> Self {
        Keymap { inner }
    }

    pub fn state(&self) -> State {
        unsafe { State::new(xkb_state_new(self.inner)) }
    }
}

impl Clone for Keymap {
    fn clone(&self) -> Self {
        unsafe {
            xkb_keymap_ref(self.inner);
            Self { inner: self.inner }
        }
    }
}

impl Drop for Keymap {
    fn drop(&mut self) {
        unsafe {
            xkb_keymap_unref(self.inner);
        }
    }
}

pub struct State {
    inner: *mut xkb_state,
}

impl State {
    unsafe fn new(inner: *mut xkb_state) -> Self {
        Self { inner }
    }

    pub fn key_event(&self, scancode: u32, state: KeyState, mods: Modifiers) -> KeyEvent {
        let scancode = scancode + 8;
        let code = u16::try_from(scancode)
            .map(hardware_keycode_to_code)
            .unwrap_or(Code::Unidentified);
        let key = self.get_logical_key(scancode);
        // TODO this is lazy - really should use xkb i.e. augment the get_logical_key method.
        let location = code_to_location(code);
        // TODO rest are unimplemented
        let repeat = false;
        // TODO we get the information for this from a wayland event
        let is_composing = false;

        // Update xkb's state (e.g. return capitals if we've pressed shift)
        unsafe {
            xkb_state_update_key(
                self.inner,
                scancode,
                match state {
                    KeyState::Down => XKB_KEY_DOWN,
                    KeyState::Up => XKB_KEY_UP,
                },
            );
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
        unsafe { xkb_state_key_get_one_sym(self.inner, scancode) }
    }

    /// Get the string representation of a key.
    // TODO `keyboard_types` forces us to return a String, but it would be nicer if we could stay
    // on the stack, especially since we expect most results to be pretty small.
    fn key_get_utf8(&self, scancode: u32) -> Option<String> {
        unsafe {
            // First get the size we will need
            let len = xkb_state_key_get_utf8(self.inner, scancode, ptr::null_mut(), 0);
            if len == 0 {
                return None;
            }
            // add 1 because we will get a null-terminated string.
            let len = usize::try_from(len).unwrap() + 1;
            let mut buf: Vec<u8> = Vec::new();
            buf.resize(len, 0);
            xkb_state_key_get_utf8(self.inner, scancode, buf.as_mut_ptr() as *mut i8, len);
            debug_assert!(buf[buf.len() - 1] == 0);
            buf.pop();
            Some(String::from_utf8(buf).unwrap())
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        unsafe {
            xkb_state_ref(self.inner);
            Self { inner: self.inner }
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            xkb_state_unref(self.inner);
        }
    }
}

/// Map from an xkb_common key code to a key, if possible.
fn map_key(keysym: u32) -> Key {
    use Key::*;
    match keysym {
        XKB_KEY_Alt_L => Alt,
        XKB_KEY_Alt_R => AltGraph,
        XKB_KEY_Caps_Lock => CapsLock,
        XKB_KEY_Control_L | XKB_KEY_Control_R => Control,
        XKB_KEY_Escape => Escape,
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
        XKB_KEY_ISO_Left_Tab => Tab,
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
        // Copy,
        XKB_KEY_3270_CursorSelect => CrSel,
        //Cut,
        //Delete,
        //EraseEof,
        //ExSel,
        XKB_KEY_Insert => Insert,
        //Paste,
        XKB_KEY_Redo => Redo,
        XKB_KEY_Undo => Undo,
        /* todo carry on
        Accept,
        Again,
        Attn,
        Cancel,
        ContextMenu,
        Execute,
        Find,
        Help,
        Pause,
        Play,
        Props,
        Select,
        ZoomIn,
        ZoomOut,
        BrightnessDown,
        BrightnessUp,
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
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
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
