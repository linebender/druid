// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This is a demo of the settings of Slider, RangeSlider and Annotated.
//! It contains a `Slider` and `RangeSlider`.
//! Every time the `RangeSlider` is moved the range of the `Slider` is updated.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::prelude::*;
use druid::widget::{
    Axis, CrossAxisAlignment, Flex, KnobStyle, Label, RangeSlider, Slider, ViewSwitcher,
};
use druid::{AppLauncher, Color, Data, KeyOrValue, Lens, UnitPoint, WidgetExt, WindowDesc};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;

#[derive(Clone, Data, Lens)]
struct AppState {
    range: (f64, f64),
    value: f64,
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title("Slider Demo!")
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state: AppState = AppState {
        range: (2.0, 8.0),
        value: 5.0,
    };

    // start the application. Here we pass in the application state.
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    let range = Flex::row()
        .with_child(Label::dynamic(|value: &(f64, f64), _| {
            format!("Value Range: {value:?}")
        }))
        .with_default_spacer()
        .with_child(
            RangeSlider::new()
                .with_range(0.0, 20.0)
                .with_step(1.0)
                .track_color(KeyOrValue::Concrete(Color::RED))
                .fix_width(250.0),
        )
        .lens(AppState::range);

    let value = Flex::row()
        .with_child(Label::dynamic(|value: &AppState, _| {
            format!("Value: {:?}", value.value)
        }))
        .with_default_spacer()
        .with_child(ViewSwitcher::new(
            |data: &AppState, _| data.range,
            |range, _, _| {
                Slider::new()
                    .with_range(range.0, range.1)
                    .track_color(KeyOrValue::Concrete(Color::RED))
                    .knob_style(KnobStyle::Wedge)
                    .axis(Axis::Vertical)
                    .with_step(0.25)
                    .annotated(1.0, 0.25)
                    .fix_height(250.0)
                    .lens(AppState::value)
                    .boxed()
            },
        ));

    // arrange the two widgets vertically, with some padding
    Flex::column()
        .with_child(range)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(value)
        .cross_axis_alignment(CrossAxisAlignment::End)
        .align_vertical(UnitPoint::RIGHT)
        .padding(20.0)
}
