// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A minimal wrapper around Xkb for our use.

mod keycodes;
mod xkbcommon_sys;
use crate::{
    backend::shared::{code_to_location, hardware_keycode_to_code},
    KeyEvent, KeyState, Modifiers,
};
use keyboard_types::{Code, Key};
use std::convert::TryFrom;
use std::os::raw::c_char;
use std::ptr;
use xkbcommon_sys::*;

#[cfg(feature = "x11")]
use x11rb::xcb_ffi::XCBConnection;

#[cfg(feature = "x11")]
pub struct DeviceId(std::os::raw::c_int);

/// A global xkb context object.
///
/// Reference counted under the hood.
// Assume this isn't threadsafe unless proved otherwise. (e.g. don't implement Send/Sync)
pub struct Context(*mut xkb_context);

impl Context {
    /// Create a new xkb context.
    ///
    /// The returned object is lightweight and clones will point at the same context internally.
    pub fn new() -> Self {
        unsafe { Self(xkb_context_new(XKB_CONTEXT_NO_FLAGS)) }
    }

    #[cfg(feature = "x11")]
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

    #[cfg(feature = "x11")]
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

    /// Create a keymap from some given data.
    ///
    /// Uses `xkb_keymap_new_from_buffer` under the hood.
    #[cfg(feature = "wayland")]
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
                self.0,
                buffer.as_ptr() as *const i8,
                XKB_KEYMAP_FORMAT_TEXT_V1,
                XKB_KEYMAP_COMPILE_NO_FLAGS,
            );
            assert!(!keymap.is_null());
            Keymap(keymap)
        }
    }

    /// Set the log level using `tracing` levels.
    ///
    /// Because `xkb` has a `critical` error, each rust error maps to 1 above (e.g. error ->
    /// critical, warn -> error etc.)
    #[allow(unused)]
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
        State::new(self)
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

pub struct State {
    state: *mut xkb_state,
    mods: ModsIndices,
}

#[derive(Clone, Copy)]
pub struct ModsIndices {
    control: xkb_mod_index_t,
    shift: xkb_mod_index_t,
    alt: xkb_mod_index_t,
    super_: xkb_mod_index_t,
    caps_lock: xkb_mod_index_t,
    num_lock: xkb_mod_index_t,
}

impl State {
    pub fn new(keymap: &Keymap) -> Self {
        let keymap = keymap.0;
        let state = unsafe { xkb_state_new(keymap) };
        let mod_idx = |str: &'static [u8]| unsafe {
            xkb_keymap_mod_get_index(keymap, str.as_ptr() as *mut c_char)
        };
        Self {
            state,
            mods: ModsIndices {
                control: mod_idx(XKB_MOD_NAME_CTRL),
                shift: mod_idx(XKB_MOD_NAME_SHIFT),
                alt: mod_idx(XKB_MOD_NAME_ALT),
                super_: mod_idx(XKB_MOD_NAME_LOGO),
                caps_lock: mod_idx(XKB_MOD_NAME_CAPS),
                num_lock: mod_idx(XKB_MOD_NAME_NUM),
            },
        }
    }

    pub fn key_event(&mut self, scancode: u32, state: KeyState, repeat: bool) -> KeyEvent {
        let code = u16::try_from(scancode)
            .map(hardware_keycode_to_code)
            .unwrap_or(Code::Unidentified);
        let key = self.get_logical_key(scancode);
        // TODO this is lazy - really should use xkb i.e. augment the get_logical_key method.
        let location = code_to_location(code);

        // TODO not sure how to get this
        let is_composing = false;

        let mut mods = Modifiers::empty();
        // Update xkb's state (e.g. return capitals if we've pressed shift)
        unsafe {
            if !repeat {
                xkb_state_update_key(
                    self.state,
                    scancode,
                    match state {
                        KeyState::Down => XKB_KEY_DOWN,
                        KeyState::Up => XKB_KEY_UP,
                    },
                );
            }
            // compiler will unroll this loop
            // FIXME(msrv): remove .iter().cloned() when msrv is >= 1.53
            for (idx, mod_) in [
                (self.mods.control, Modifiers::CONTROL),
                (self.mods.shift, Modifiers::SHIFT),
                (self.mods.super_, Modifiers::SUPER),
                (self.mods.alt, Modifiers::ALT),
                (self.mods.caps_lock, Modifiers::CAPS_LOCK),
                (self.mods.num_lock, Modifiers::NUM_LOCK),
            ]
            .iter()
            .cloned()
            {
                if xkb_state_mod_index_is_active(self.state, idx, XKB_STATE_MODS_EFFECTIVE) != 0 {
                    mods |= mod_;
                }
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

    fn get_logical_key(&mut self, scancode: u32) -> Key {
        let mut key = keycodes::map_key(self.key_get_one_sym(scancode));
        if matches!(key, Key::Unidentified) {
            if let Some(s) = self.key_get_utf8(scancode) {
                key = Key::Character(s);
            }
        }
        key
    }

    fn key_get_one_sym(&mut self, scancode: u32) -> u32 {
        unsafe { xkb_state_key_get_one_sym(self.state, scancode) }
    }

    /// Get the string representation of a key.
    // TODO `keyboard_types` forces us to return a String, but it would be nicer if we could stay
    // on the stack, especially since we expect most results to be pretty small.
    fn key_get_utf8(&mut self, scancode: u32) -> Option<String> {
        unsafe {
            // First get the size we will need
            let len = xkb_state_key_get_utf8(self.state, scancode, ptr::null_mut(), 0);
            if len == 0 {
                return None;
            }
            // add 1 because we will get a null-terminated string.
            let len = usize::try_from(len).unwrap() + 1;
            let mut buf: Vec<u8> = vec![0; len];
            xkb_state_key_get_utf8(self.state, scancode, buf.as_mut_ptr() as *mut c_char, len);
            assert!(buf[buf.len() - 1] == 0);
            buf.pop();
            Some(String::from_utf8(buf).unwrap())
        }
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            state: unsafe { xkb_state_ref(self.state) },
            mods: self.mods,
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            xkb_state_unref(self.state);
        }
    }
}
