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

pub mod application;
mod buffer;
pub mod clipboard;
pub mod dialog;
pub mod error;
mod events;
pub mod keycodes;
pub mod menu;
mod pointer;
pub mod screen;
pub mod util;
pub mod window;
mod xkb;

/// Number of bytes for a pixel (argb = 4)
const PIXEL_WIDTH: i32 = 4;
/// Number of frames we need (2 for double buffering)
const NUM_FRAMES: i32 = 2;

/// Little enum to make it clearer what some return values mean.
#[derive(Copy, Clone)]
enum Changed {
    Changed,
    Unchanged,
}

impl Changed {
    fn is_changed(self) -> bool {
        matches!(self, Changed::Changed)
    }
}
