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

use druid::widget::{Align, Flex, Label, Parse, TextBox};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title(
        LocalizedString::new("parse-demo-window-title").with_placeholder("Number Parsing Demo"),
    );
    let data = Some(0);
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<Option<u32>> {
    let label = Label::new(|data: &Option<u32>, _env: &_| {
        data.map_or_else(|| "Invalid input".into(), |x| x.to_string())
    });
    let input = Parse::new(TextBox::new());

    let mut col = Flex::column();
    col.add_child(label);
    col.add_spacer(8.0);
    col.add_child(input);
    Align::centered(col)
}
