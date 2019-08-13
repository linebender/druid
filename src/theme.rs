// Copyright 2019 The xi-editor Authors.
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

//! Theme keys and initial values.

use crate::piet::Color;

use crate::{Env, Key};

pub const BACKGROUND_COLOR: Key<Color> = Key::new("background_color");
pub const HOVER_COLOR: Key<Color> = Key::new("hover_color");
pub const PRESSED_COLOR: Key<Color> = Key::new("pressed_color");
pub const LABEL_COLOR: Key<Color> = Key::new("label_color");
pub const FONT_NAME: Key<&str> = Key::new("font_name");
pub const TEXT_SIZE_NORMAL: Key<f64> = Key::new("text_size_normal");

/// An initial theme.
pub fn init() -> Env {
    let mut env = Env::default()
        .adding(BACKGROUND_COLOR, Color::rgb8(0x40, 0x40, 0x48))
        .adding(HOVER_COLOR, Color::rgb8(0x50, 0x50, 0x58))
        .adding(PRESSED_COLOR, Color::rgb8(0x60, 0x60, 0x68))
        .adding(LABEL_COLOR, Color::rgb8(0xf0, 0xf0, 0xea))
        .adding(TEXT_SIZE_NORMAL, 15.0);

    #[cfg(target_os = "windows")]
    {
        env = env.adding(FONT_NAME, "Segoe UI");
    }
    #[cfg(target_os = "macos")]
    {
        // Ideally this would be a reference to San Francisco, but Cairo's
        // "toy text" API doesn't seem to be able to access it easily.
        env = env.adding(FONT_NAME, "Arial");
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        env = env.adding(FONT_NAME, "sans-serif");
    }
    env
}
