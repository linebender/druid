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

//! This is an example of arbitrary transform of widgets.

use druid::widget::{AARotation, AATransformBox, Button, Flex, Radio, ViewSwitcher, TransformBox};
use druid::{Env, WidgetExt, Widget, Affine, WindowDesc, AppLauncher, Data, Lens};

#[derive(Clone, Data, Lens)]
struct TransformState {
    rotation: AARotation,
}

fn rotated_widget(data: &TransformState, _: &TransformState, _: &Env) -> Box<dyn Widget<TransformState>> {
    AATransformBox::new(
    Button::new("Rotatable test button!")
            .fix_width(300.0)
            .padding(50.0)
    ).rotated(data.rotation)
        .boxed()
}

fn build_root_widget() -> impl Widget<TransformState> {
    let settings = Flex::column()
        .with_child(Radio::new("0°", AARotation::ORIGIN))
        .with_default_spacer()
        .with_child(Radio::new("90°", AARotation::CLOCKWISE))
        .with_default_spacer()
        .with_child(Radio::new("180°", AARotation::HALF_WAY))
        .with_default_spacer()
        .with_child(Radio::new("270°", AARotation::COUNTER_CLOCKWISE));

    Flex::row()
        .with_child(settings.lens(TransformState::rotation))
        .with_default_spacer()
        .with_child(ViewSwitcher::<TransformState, _>::new(
            |data: &TransformState, _|data.to_owned(),
            rotated_widget
            ))
        .with_default_spacer()
        .with_child(TransformBox::with_transform(
            Button::new("test sting data"),
            Affine::rotate(0.6)
        ))
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title("Transform example!")
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state: TransformState = TransformState {
        rotation: AARotation::ORIGIN,
    };

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}
