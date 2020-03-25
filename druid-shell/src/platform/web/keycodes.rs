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

//! Web keycode handling.

//use web_sys::*;

use crate::keycodes::KeyCode;

pub type RawKeyCode = u32;

//// TODO: Verify if scancodes are indeed OS specific even from wasm.
//mod hardware {
//    cfg_if::cfg_if! {
//        if #[cfg(all(target_os = "windows", not(feature = "use_gtk")))] {
//            pub use super::windows::keycodes::*;
//        } else if #[cfg(all(target_os = "macos", not(feature = "use_gtk")))] {
//            pub use super::mac::keycodes::*;
//        } else if #[cfg(any(feature = "use_gtk", target_os = "linux"))] {
//            pub use super::gtk::keycodes::*;
//        }
//    }
//}
//
//impl From<KeyCode> for u32 {
//    fn from(src: KeyCode) -> u32 {
//        hardware::RawKeyCode::from(src) as u32
//    }
//}
//
//impl From<u32> for KeyCode {
//    fn from(src: u32) -> KeyCode {
//        KeyCode::from(src as hardware::RawKeyCode)
//    }
//}
