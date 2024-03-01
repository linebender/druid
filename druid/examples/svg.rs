// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This example shows how to draw an SVG.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use tracing::error;

use druid::{
    widget::{Flex, Svg, SvgData, WidgetExt},
    AppLauncher, LocalizedString, Widget, WindowDesc,
};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder())
        .title(LocalizedString::new("svg-demo-window-title").with_placeholder("Rawr!"));
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .log_to_console()
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
    col.add_flex_child(Svg::new(tiger_svg.clone()), 1.0);
    col.add_flex_child(Svg::new(tiger_svg), 1.0);
    col.debug_paint_layout()
}
