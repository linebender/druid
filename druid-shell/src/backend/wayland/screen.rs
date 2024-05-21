// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! wayland Monitors and Screen information.

use crate::kurbo::Rect;

use crate::screen::Monitor;

use super::error;
use super::outputs;

fn _get_monitors() -> Result<Vec<Monitor>, error::Error> {
    let metas = outputs::current()?;
    let monitors: Vec<Monitor> = metas
        .iter()
        .map(|m| {
            let rect = Rect::from_origin_size(
                (m.position.x as f64, m.position.y as f64),
                (m.logical.width as f64, m.logical.height as f64),
            );
            Monitor::new(false, rect, rect)
        })
        .collect();
    Ok(monitors)
}

pub(crate) fn get_monitors() -> Vec<Monitor> {
    match _get_monitors() {
        Ok(m) => m,
        Err(cause) => {
            tracing::error!(
                "unable to detect monitors, failed to connect to wayland server {:?}",
                cause
            );
            Vec::new()
        }
    }
}
