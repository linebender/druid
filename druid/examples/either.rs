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

use druid::widget::{Checkbox, Either, Flex, Label, Slider};
use druid::{AppLauncher, Data, Lens, LocalizedString, Widget, WidgetExt, WindowDesc};

#[derive(Clone, Default, Data, Lens)]
struct AppState {
    which: bool,
    value: f64,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title(
        LocalizedString::new("either-demo-window-title")
            .with_placeholder("Switcheroo")
            .with_arg("view", |data: &AppState, _env| (data.which as u8).into()),
    );
    let data = AppState::default();
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppState> {
    let label = Label::new("Click to reveal slider");

    let mut col = Flex::column();
    col.add_child(
        Checkbox::new("Toggle slider")
            .lens(AppState::which)
            .padding(5.0),
    );
    let either = Either::new(
        |data, _env| data.which,
        Slider::new().lens(AppState::value).padding(5.0),
        label.padding(5.0),
    );
    col.add_child(either);
    col
}
