// Copyright 2019 The Druid Authors.
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

//! Platform specific implementations.

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        pub use windows::*;
    } else if #[cfg(target_os = "macos")] {
        mod mac;
        pub use mac::*;
        pub(crate) mod shared;
    } else if #[cfg(all(feature = "x11", target_os = "linux"))] {
        mod x11;
        pub use x11::*;
        pub(crate) mod shared;
    } else if #[cfg(target_os = "linux")] {
        mod gtk;
        pub use self::gtk::*;
        pub(crate) mod shared;
    } else if #[cfg(target_arch = "wasm32")] {
        mod web;
        pub use web::*;
    }
}
