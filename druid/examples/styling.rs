// Copyright 2020 The Druid Authors.
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

use druid::widget::{button, Button, Flex};
use druid::{AppLauncher, Data, Lens, UnitPoint, Widget, WidgetExt, WindowDesc};

#[derive(Clone, Data, Lens)]
struct AppState {}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title("Styling")
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = AppState {};

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

struct ButtonStyleSheet;

impl button::StyleSheet for ButtonStyleSheet {
    fn normal(&self) -> button::Style {
        button::Style {
            border_width: (0.).into(),
            background_is_gradient: false.into(),
            background_color_a: druid::theme::PRIMARY_DARK.into(),
            ..button::Style::default()
        }
    }

    fn hot(&self) -> button::Style {
        let normal = self.normal();

        button::Style {
            background_color_a: druid::theme::PRIMARY_LIGHT.into(),
            ..normal
        }
    }

    fn active(&self) -> button::Style {
        let hot = self.hot();
        button::Style {
            background_color_a: druid::theme::BUTTON_DARK.into(),
            ..hot
        }
    }
}

fn build_root_widget() -> impl Widget<AppState> {
    let button = Button::new("Heyo");
    let styled_button = Button::new("Heyo").with_style(ButtonStyleSheet {});

    Flex::column()
        .with_child(button)
        .with_spacer(20.)
        .with_child(styled_button)
        .align_vertical(UnitPoint::CENTER)
}
