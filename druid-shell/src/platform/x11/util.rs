// Copyright 2020 The Druid Authors.
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

use std::cmp::Ordering;
use std::rc::Rc;
use std::time::Instant;

use anyhow::{anyhow, Error};
use x11rb::protocol::randr::{ConnectionExt, ModeFlag};
use x11rb::protocol::xproto::{Screen, Visualtype, Window};
use x11rb::xcb_ffi::XCBConnection;

use crate::window::TimerToken;

// See: https://github.com/rtbo/rust-xcb/blob/master/examples/randr_screen_modes.rs
pub fn refresh_rate(conn: &Rc<XCBConnection>, window_id: Window) -> Option<f64> {
    let try_refresh_rate = || -> Result<f64, Error> {
        let reply = conn.randr_get_screen_resources(window_id)?.reply()?;

        // TODO(x11/render_improvements): Figure out a more correct way of getting the screen's refresh rate.
        //     Or maybe we don't even need this function if I figure out a better way to schedule redraws?
        //     Assuming the first mode is the one we want to use. This is probably a bug on some setups.
        //     Any better way to find the correct one?
        reply
            .modes
            .first()
            .ok_or_else(|| anyhow!("didn't get any modes"))
            .and_then(|mode_info| {
                let flags = mode_info.mode_flags;
                let vtotal = {
                    let mut val = mode_info.vtotal;
                    if (flags & u32::from(ModeFlag::DoubleScan)) != 0 {
                        val *= 2;
                    }
                    if (flags & u32::from(ModeFlag::Interlace)) != 0 {
                        val /= 2;
                    }
                    val
                };

                if vtotal != 0 && mode_info.htotal != 0 {
                    Ok((mode_info.dot_clock as f64) / (vtotal as f64 * mode_info.htotal as f64))
                } else {
                    Err(anyhow!("got nonsensical mode values"))
                }
            })
    };

    match try_refresh_rate() {
        Err(e) => {
            log::error!("failed to find refresh rate: {}", e);
            None
        }
        Ok(r) => Some(r),
    }
}

// Apparently you have to get the visualtype this way :|
pub fn get_visual_from_screen(screen: &Screen) -> Option<Visualtype> {
    for depth in &screen.allowed_depths {
        for visual in &depth.visuals {
            if visual.visual_id == screen.root_visual {
                return Some(*visual);
            }
        }
    }
    None
}

macro_rules! log_x11 {
    ($val:expr) => {
        if let Err(e) = $val {
            // We probably don't want to include file/line numbers. This logging is done in
            // a context where X11 errors probably just mean that the connection to the X server
            // was lost. In particular, it doesn't represent a druid-shell bug for which we want
            // more context.
            log::error!("X11 error: {}", e);
        }
    };
}

/// A timer is a deadline (`std::Time::Instant`) and a `TimerToken`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timer {
    deadline: Instant,
    token: TimerToken,
}

impl Timer {
    pub(crate) fn new(deadline: Instant) -> Self {
        let token = TimerToken::next();
        Self { deadline, token }
    }

    pub(crate) fn deadline(&self) -> Instant {
        self.deadline
    }

    pub(crate) fn token(&self) -> TimerToken {
        self.token
    }
}

impl Ord for Timer {
    /// Ordering is so that earliest deadline sorts first
    // "Earliest deadline first" that a std::collections::BinaryHeap will have the earliest timer
    // at its head, which is just what is needed for timer management.
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline.cmp(&other.deadline).reverse()
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
