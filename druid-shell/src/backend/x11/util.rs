// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Miscellaneous utility functions for working with X11.

use std::rc::Rc;

use anyhow::{anyhow, Error};
use x11rb::connection::RequestConnection;
use x11rb::errors::ReplyError;
use x11rb::protocol::randr::{ConnectionExt, ModeFlag};
use x11rb::protocol::render::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{Screen, Visualid, Visualtype, Window};
use x11rb::xcb_ffi::XCBConnection;

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
                    if (flags & u32::from(ModeFlag::DOUBLE_SCAN)) != 0 {
                        val *= 2;
                    }
                    if (flags & u32::from(ModeFlag::INTERLACE)) != 0 {
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
            tracing::error!("failed to find refresh rate: {}", e);
            None
        }
        Ok(r) => Some(r),
    }
}

// Apparently you have to get the visualtype this way :|
fn find_visual_from_screen(screen: &Screen, visual_id: u32) -> Option<Visualtype> {
    for depth in &screen.allowed_depths {
        for visual in &depth.visuals {
            if visual.visual_id == visual_id {
                return Some(*visual);
            }
        }
    }
    None
}

pub fn get_visual_from_screen(screen: &Screen) -> Option<Visualtype> {
    find_visual_from_screen(screen, screen.root_visual)
}

pub fn get_argb_visual_type(
    conn: &XCBConnection,
    screen: &Screen,
) -> Result<Option<Visualtype>, ReplyError> {
    fn find_visual_for_format(
        reply: &render::QueryPictFormatsReply,
        id: render::Pictformat,
    ) -> Option<Visualid> {
        let find_in_depth = |depth: &render::Pictdepth| {
            depth
                .visuals
                .iter()
                .find(|visual| visual.format == id)
                .map(|visual| visual.visual)
        };
        let find_in_screen =
            |screen: &render::Pictscreen| screen.depths.iter().find_map(find_in_depth);
        reply.screens.iter().find_map(find_in_screen)
    }

    // Getting a visual is already funny, but finding the ARGB32 visual is even more fun.
    // RENDER has picture formats. Each format corresponds to a visual. Thus, we first find the
    // right picture format, then find the corresponding visual id, then the Visualtype.
    if conn
        .extension_information(render::X11_EXTENSION_NAME)?
        .is_none()
    {
        // RENDER not supported
        Ok(None)
    } else {
        let pict_formats = conn.render_query_pict_formats()?.reply()?;
        // Find the ARGB32 standard format
        let res = pict_formats
            .formats
            .iter()
            .find(|format| {
                format.type_ == render::PictType::DIRECT
                    && format.depth == 32
                    && format.direct.red_shift == 16
                    && format.direct.red_mask == 0xff
                    && format.direct.green_shift == 8
                    && format.direct.green_mask == 0xff
                    && format.direct.blue_shift == 0
                    && format.direct.blue_mask == 0xff
                    && format.direct.alpha_shift == 24
                    && format.direct.alpha_mask == 0xff
            })
            // Now find the corresponding visual ID
            .and_then(|format| find_visual_for_format(&pict_formats, format.id))
            // And finally, we can find the visual
            .and_then(|visual_id| find_visual_from_screen(screen, visual_id));
        Ok(res)
    }
}

macro_rules! log_x11 {
    ($val:expr) => {
        if let Err(e) = $val {
            // We probably don't want to include file/line numbers. This logging is done in
            // a context where X11 errors probably just mean that the connection to the X server
            // was lost. In particular, it doesn't represent a druid-shell bug for which we want
            // more context.
            tracing::error!("X11 error: {}", e);
        }
    };
}
