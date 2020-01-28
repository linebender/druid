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

use druid::widget::Slider;
use druid::widget::{Flex, TextBox, WidgetExt};
use druid::{AppLauncher, Data, Lens, LocalizedString, Widget, WindowDesc};

fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("lens-demo-window-title").with_placeholder("Lens Demo"));
    let data = MyComplexState {
        term: String::new(),
        scale: 0.0,
    };

    AppLauncher::with_window(main_window)
        .launch(data)
        .expect("launch failed");
}

#[derive(Clone, Debug, Data, Lens)]
struct MyComplexState {
    term: String,
    scale: f64,
}

fn ui_builder() -> impl Widget<MyComplexState> {
    // `TextBox` is of type `Widget<String>`
    // via `.lens` we get it to be of type `Widget<MyComplexState>`
    let searchbar = TextBox::new().lens(MyComplexState::term);

    // `Slider` is of type `Widget<f64>`
    // via `.lens` we get it to be of type `Widget<MyComplexState>`
    let slider = Slider::new().lens(MyComplexState::scale);

    Flex::column()
        .with_child(searchbar.padding(32.0), 1.0)
        .with_child(slider.padding(32.0), 1.0)
}
