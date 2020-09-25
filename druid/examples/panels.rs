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

//! This example shows how to construct a basic layout.

use druid::kurbo::Circle;
use druid::widget::{Flex, Label, Painter};
use druid::{
    AppLauncher, Color, LinearGradient, LocalizedString, PlatformError, RenderContext, UnitPoint,
    Widget, WidgetExt, WindowDesc,
};

const DARK_GREY: Color = Color::grey8(0x3a);
const DARKER_GREY: Color = Color::grey8(0x11);
const LIGHTER_GREY: Color = Color::grey8(0xbb);

fn build_app() -> impl Widget<()> {
    let gradient = LinearGradient::new(
        UnitPoint::TOP_LEFT,
        UnitPoint::BOTTOM_RIGHT,
        (DARKER_GREY, LIGHTER_GREY),
    );

    // a custom background
    let polka_dots = Painter::new(|ctx, _, _| {
        let bounds = ctx.size().to_rect();
        let dot_diam = bounds.width().max(bounds.height()) / 20.;
        let dot_spacing = dot_diam * 1.8;
        for y in 0..((bounds.height() / dot_diam).ceil() as usize) {
            for x in 0..((bounds.width() / dot_diam).ceil() as usize) {
                let x_offset = (y % 2) as f64 * (dot_spacing / 2.0);
                let x = x as f64 * dot_spacing + x_offset;
                let y = y as f64 * dot_spacing;
                let circ = Circle::new((x, y), dot_diam / 2.0);
                let purp = Color::rgb(1.0, 0.22, 0.76);
                ctx.fill(circ, &purp);
            }
        }
    });

    Flex::column()
        .with_flex_child(
            Flex::row()
                .with_flex_child(
                    Label::new("top left")
                        .center()
                        .border(DARK_GREY, 4.0)
                        .padding(10.0),
                    1.0,
                )
                .with_flex_child(
                    Label::new("top right")
                        .center()
                        .background(DARK_GREY)
                        .padding(10.0),
                    1.0,
                ),
            1.0,
        )
        .with_flex_child(
            Flex::row()
                .with_flex_child(
                    Label::new("bottom left")
                        .center()
                        .background(gradient)
                        .rounded(10.0)
                        .padding(10.0),
                    1.0,
                )
                .with_flex_child(
                    Label::new("bottom right")
                        .center()
                        .border(LIGHTER_GREY, 4.0)
                        .background(polka_dots)
                        .rounded(10.0)
                        .padding(10.0),
                    1.0,
                ),
            1.0,
        )
}

pub fn main() -> Result<(), PlatformError> {
    let main_window = WindowDesc::new(build_app)
        .title(LocalizedString::new("panels-demo-window-title").with_placeholder("Fancy Boxes!"));
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(())?;

    Ok(())
}
