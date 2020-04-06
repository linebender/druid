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

//! X11 implementation of druid-shell.

// TODO(x11/render_improvements): screen is currently flashing when resizing in perftest.
//     Might be related to the "sleep scheduler" in XWindow::render()?
// TODO(x11/render_improvements): double-buffering / present strategies / etc?

pub mod application;
pub mod clipboard;
pub mod error;
pub mod keycodes;
pub mod menu;
pub mod window;

mod util;
