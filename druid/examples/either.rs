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

use druid::widget::{Checkbox, Either, Flex, Label, Padding, Slider};
use druid::{AppLauncher, Data, Lens, LensWrap, Widget, WindowDesc};

#[derive(Clone, Default, Data, Lens)]
struct AppState {
    which: bool,
    value: f64,
}

fn main() {
    let main_window = WindowDesc::new(ui_builder);
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
        Padding::new(
            5.0,
            LensWrap::new(Checkbox::new(), lenses::app_state::which),
        ),
        0.0,
    );
    let either = Either::new(
        |data, _env| data.which,
        Padding::new(5.0, LensWrap::new(Slider::new(), lenses::app_state::value)),
        Padding::new(5.0, label),
    );
    col.add_child(either, 0.0);
    col
}
