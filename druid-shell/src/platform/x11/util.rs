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

//! Miscellaneous utility functions for working with X11.

use xcb::{Connection, Screen, Visualtype, Window};

// See: https://github.com/rtbo/rust-xcb/blob/master/examples/randr_screen_modes.rs
pub fn refresh_rate(conn: &Connection, window_id: Window) -> Option<f64> {
    let cookie = xcb::randr::get_screen_resources(conn, window_id);
    let reply = cookie.get_reply().unwrap();
    let mut modes = reply.modes();

    // TODO(x11/render_improvements): Figure out a more correct way of getting the screen's refresh rate.
    //     Or maybe we don't even need this function if I figure out a better way to schedule redraws?
    //     Assuming the first mode is the one we want to use. This is probably a bug on some setups.
    //     Any better way to find the correct one?
    let refresh_rate = modes.next().and_then(|mode_info| {
        let flags = mode_info.mode_flags();
        let vtotal = {
            let mut val = mode_info.vtotal();
            if (flags & xcb::randr::MODE_FLAG_DOUBLE_SCAN) != 0 {
                val *= 2;
            }
            if (flags & xcb::randr::MODE_FLAG_INTERLACE) != 0 {
                val /= 2;
            }
            val
        };

        if vtotal != 0 && mode_info.htotal() != 0 {
            Some((mode_info.dot_clock() as f64) / (vtotal as f64 * mode_info.htotal() as f64))
        } else {
            None
        }
    })?;

    Some(refresh_rate)
}

// Apparently you have to get the visualtype this way :|
pub fn get_visual_from_screen(screen: &Screen<'_>) -> Option<Visualtype> {
    for depth in screen.allowed_depths() {
        for visual in depth.visuals() {
            if visual.visual_id() == screen.root_visual() {
                return Some(visual);
            }
        }
    }
    None
}
