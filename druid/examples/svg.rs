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

//! This example shows how to draw an SVG.

use log::error;

use druid::{
    widget::{FillStrat, Flex, Svg, SvgData, WidgetExt},
    AppLauncher, LocalizedString, Widget, WindowDesc,
};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("svg-demo-window-title").with_placeholder("Rawr!"));
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<u32> {
    let tiger_svg = match include_str!("./assets/tiger.svg").parse::<SvgData>() {
        Ok(svg) => svg,
        Err(err) => {
            error!("{}", err);
            error!("Using an empty SVG instead.");
            SvgData::default()
        }
    };

    let mut col = Flex::column();

    col.add_flex_child(Svg::new(tiger_svg.clone()).fix_width(60.0).center(), 1.0);
    col.add_flex_child(Svg::new(tiger_svg.clone()).fill_mode(FillStrat::Fill), 1.0);
    col.add_flex_child(Svg::new(tiger_svg), 1.0);
    col.debug_paint_layout()
}
