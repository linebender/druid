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

//! This example allows to play with scroll bars over different color tones.

use druid::widget::{Container, Flex, Scroll, SizedBox};
use druid::{AppLauncher, Color, LocalizedString, Widget, WindowDesc};

fn build_app() -> impl Widget<u32> {
    let mut col = Flex::column();
    let rows = 30;
    let cols = 30;

    for i in 0..cols {
        let mut row = Flex::row();
        let col_progress = i as f64 / cols as f64;

        for j in 0..rows {
            let row_progress = j as f64 / rows as f64;

            row.add_child(
                Container::new(SizedBox::empty().width(200.0).height(200.0))
                    .background(Color::rgb(1.0 * col_progress, 1.0 * row_progress, 1.0)),
            );
        }

        col.add_child(row);
    }

    Scroll::new(col)
}

pub fn main() {
    let main_window = WindowDesc::new(build_app).title(
        LocalizedString::new("scroll-colors-demo-window-title").with_placeholder("Rainbows!"),
    );
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}
