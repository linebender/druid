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

//! This example shows how to construct a basic layout.

use druid::widget::{Flex, Label, WidgetExt};
use druid::{AppLauncher, Color, LinearGradient, PlatformError, UnitPoint, Widget, WindowDesc};

fn build_app() -> impl Widget<()> {
    let solid = Color::rgb8(0x3a, 0x3a, 0x3a);
    let gradient = LinearGradient::new(
        UnitPoint::TOP_LEFT,
        UnitPoint::BOTTOM_RIGHT,
        (Color::rgb8(0x11, 0x11, 0x11), Color::rgb8(0xbb, 0xbb, 0xbb)),
    );

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(
                    Label::new("top left")
                        .border(gradient.clone(), 4.0)
                        .padding(10.0),
                    1.0,
                )
                .with_child(
                    Label::new("top right")
                        .background(solid.clone())
                        .padding(10.0),
                    1.0,
                ),
            1.0,
        )
        .with_child(
            Flex::row()
                .with_child(
                    Label::new("bottom left")
                        .background(gradient.clone())
                        .rounded(10.0)
                        .padding(10.0),
                    1.0,
                )
                .with_child(
                    Label::new("bottom right")
                        .border(solid.clone(), 4.0)
                        .rounded(10.0)
                        .padding(10.0),
                    1.0,
                ),
            1.0,
        )
}

fn main() -> Result<(), PlatformError> {
    AppLauncher::with_window(WindowDesc::new(build_app))
        .use_simple_logger()
        .launch(())?;

    Ok(())
}
