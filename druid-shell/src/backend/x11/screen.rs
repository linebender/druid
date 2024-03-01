// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! X11 Monitors and Screen information.

use x11rb::connection::Connection;
use x11rb::errors::ReplyOrIdError;
use x11rb::protocol::randr::{self, ConnectionExt as _, Crtc};
use x11rb::protocol::xproto::{Screen, Timestamp};

use crate::kurbo::Rect;
use crate::screen::Monitor;

fn monitor<Pos>(primary: bool, (x, y): (Pos, Pos), (width, height): (u16, u16)) -> Monitor
where
    Pos: Into<i32>,
{
    let rect = Rect::from_points(
        (x.into() as f64, y.into() as f64),
        (width as f64, height as f64),
    );
    // TODO: Support for work_rect. It's complicated...
    Monitor::new(primary, rect, rect)
}

pub(crate) fn get_monitors() -> Vec<Monitor> {
    let result = if let Some(app) = crate::Application::try_global() {
        let app = app.backend_app;
        get_monitors_impl(app.connection().as_ref(), app.screen_num())
    } else {
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("Error in Screen::get_monitors(): {:?}", err);
                return Vec::new();
            }
        };
        get_monitors_impl(&conn, screen_num)
    };
    match result {
        Ok(monitors) => monitors,
        Err(err) => {
            tracing::error!("Error in Screen::get_monitors(): {:?}", err);
            Vec::new()
        }
    }
}

fn get_monitors_impl(
    conn: &impl Connection,
    screen_num: usize,
) -> Result<Vec<Monitor>, ReplyOrIdError> {
    let screen = &conn.setup().roots[screen_num];

    if conn
        .extension_information(randr::X11_EXTENSION_NAME)?
        .is_none()
    {
        return get_monitors_core(screen);
    }

    // Monitor support was added in RandR 1.5
    let version = conn.randr_query_version(1, 5)?.reply()?;
    match (version.major_version, version.minor_version) {
        (major, _) if major >= 2 => get_monitors_randr_monitors(conn, screen),
        (1, minor) if minor >= 5 => get_monitors_randr_monitors(conn, screen),
        (1, minor) if minor >= 3 => get_monitors_randr_screen_resources_current(conn, screen),
        (1, minor) if minor >= 2 => get_monitors_randr_screen_resources(conn, screen),
        _ => get_monitors_core(screen),
    }
}

fn get_monitors_core(screen: &Screen) -> Result<Vec<Monitor>, ReplyOrIdError> {
    Ok(vec![monitor(
        true,
        (0, 0),
        (screen.width_in_pixels, screen.height_in_pixels),
    )])
}

fn get_monitors_randr_monitors(
    conn: &impl Connection,
    screen: &Screen,
) -> Result<Vec<Monitor>, ReplyOrIdError> {
    let result = conn
        .randr_get_monitors(screen.root, true)?
        .reply()?
        .monitors
        .iter()
        .map(|info| monitor(info.primary, (info.x, info.y), (info.width, info.height)))
        .collect();
    Ok(result)
}

fn get_monitors_randr_screen_resources_current(
    conn: &impl Connection,
    screen: &Screen,
) -> Result<Vec<Monitor>, ReplyOrIdError> {
    let reply = conn
        .randr_get_screen_resources_current(screen.root)?
        .reply()?;
    get_monitors_randr_crtcs_timestamp(conn, &reply.crtcs, reply.config_timestamp)
}

fn get_monitors_randr_screen_resources(
    conn: &impl Connection,
    screen: &Screen,
) -> Result<Vec<Monitor>, ReplyOrIdError> {
    let reply = conn.randr_get_screen_resources(screen.root)?.reply()?;
    get_monitors_randr_crtcs_timestamp(conn, &reply.crtcs, reply.config_timestamp)
}

// This function first sends a number of requests, collect()ing them into a Vec and then gets the
// replies. This saves round-trips. Without the collect(), there would be one round-trip per CRTC.
#[allow(clippy::needless_collect)]
fn get_monitors_randr_crtcs_timestamp(
    conn: &impl Connection,
    crtcs: &[Crtc],
    config_timestamp: Timestamp,
) -> Result<Vec<Monitor>, ReplyOrIdError> {
    // Request information about all CRTCs
    let requests = crtcs
        .iter()
        .map(|&crtc| conn.randr_get_crtc_info(crtc, config_timestamp))
        .collect::<Vec<_>>();

    // Deal with CRTC information
    let mut result = Vec::new();
    for request in requests.into_iter() {
        let reply = request?.reply()?;
        if reply.width != 0 && reply.height != 0 {
            // First CRTC is assumed to be the primary output
            let primary = result.is_empty();
            result.push(monitor(
                primary,
                (reply.x, reply.y),
                (reply.width, reply.height),
            ));
        }
    }
    // TODO: I think we need to deduplicate monitors. In clone mode, each "clone" appears as its
    // own monitor otherwise.

    Ok(result)
}
