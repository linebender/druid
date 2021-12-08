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

//! wayland Monitors and Screen information.
use crate::screen::Monitor;

use super::error;
use super::outputs;

fn _get_monitors() -> Result<Vec<Monitor>, error::Error> {
    let metas = outputs::current()?;
    let monitors: Vec<Monitor> = metas
        .iter()
        .map(|m| {
            let rect = kurbo::Rect::from_origin_size(
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
