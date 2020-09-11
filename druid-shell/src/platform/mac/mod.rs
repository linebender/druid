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

//! macOS druid-shell backend.

#![allow(clippy::let_unit_value)]

pub mod appkit;
pub mod application;
pub mod clipboard;
pub mod dialog;
pub mod error;
mod keyboard;
pub mod menu;
pub mod util;
pub mod window;
pub mod screen;
