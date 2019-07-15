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

use druid::shell::{runloop, WindowBuilder};
use druid::widget::{ActionWrapper, Button, Column, DynLabel, Padding, ProgressBar, Slider};
use druid::{UiMain, UiState};

fn main() {

    druid_shell::init();

    let mut run_loop = runloop::RunLoop::new();
    let mut builder = WindowBuilder::new();
    
    let mut col = Column::new();
    let label_1 = DynLabel::new(|data: &f64, _env| format!("actual value: {0:.2}", data));
    let label_2 = DynLabel::new(|data: &f64, _env| format!("2x the value: {0:.2}", data * 2.0));
    let bar = ProgressBar::default();
    let slider = Slider::default();
    
    let button_1 = ActionWrapper::new(
        Button::new("increment "),
        move |data: &mut f64, _env| *data += 0.1,
    );
    let button_2 = ActionWrapper::new(
        Button::new("decrement "),
        move |data: &mut f64, _env| *data -= 0.1,
    );

    col.add_child(Padding::uniform(5.0, bar), 1.0);
    col.add_child(Padding::uniform(5.0, slider), 1.0);
    col.add_child(Padding::uniform(5.0, label_1), 1.0);
    col.add_child(Padding::uniform(5.0, label_2), 1.0);
    col.add_child(Padding::uniform(5.0, button_1), 1.0);
    col.add_child(Padding::uniform(5.0, button_2), 1.0);

    let state = UiState::new(col, 0.7f64);
    builder.set_title("Widget demo");
    builder.set_handler(Box::new(UiMain::new(state)));
    let window = builder.build().unwrap();
    window.show();
    run_loop.run();
}
