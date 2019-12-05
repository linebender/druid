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

//! This example shows how to draw an SVG.
//!
//! Requires the non-default "svg" feature to be enabled:
//! `cargo run --example svg --features "svg"`

#[cfg(not(feature = "svg"))]
fn main() {
    eprintln!("This examples requires the \"svg\" feature to be enabled:");
    eprintln!("cargo run --example svg --features \"svg\"");
}

#[cfg(feature = "svg")]
use druid::{
    widget::{Flex, Svg, WidgetExt},
    AppLauncher, Widget, WindowDesc,
};
#[cfg(feature = "svg")]
fn main() {
    let main_window = WindowDesc::new(ui_builder);
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

#[cfg(feature = "svg")]
fn ui_builder() -> impl Widget<u32> {
    let tiger_svg = include_str!("tiger.svg");
    let mut col = Flex::column();

    col.add_child(Svg::new(tiger_svg).fix_width(100.0).center(), 1.0);
    col.add_child(Svg::new(tiger_svg), 1.0);
    col
}
