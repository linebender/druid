// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! This example shows basic usage of Lens

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::Slider;
use druid::widget::{CrossAxisAlignment, Flex, Label, TextBox};
use druid::{AppLauncher, Data, Env, Lens, LocalizedString, Widget, WidgetExt, WindowDesc};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder())
        .title(LocalizedString::new("lens-demo-window-title").with_placeholder("Lens Demo"));
    let data = MyComplexState {
        term: "hello".into(),
        scale: 0.0,
    };

    AppLauncher::with_window(main_window)
        .launch(data)
        .expect("launch failed");
}

#[derive(Clone, Debug, Data, Lens)]
struct MyComplexState {
    #[lens(name = "term_lens")]
    term: String,
    scale: f64,
}

fn ui_builder() -> impl Widget<MyComplexState> {
    // `TextBox` is of type `Widget<String>`
    // via `.lens` we get it to be of type `Widget<MyComplexState>`
    let searchbar = TextBox::new().lens(MyComplexState::term_lens);

    // `Slider` is of type `Widget<f64>`
    // via `.lens` we get it to be of type `Widget<MyComplexState>`
    let slider = Slider::new().lens(MyComplexState::scale);

    let label = Label::new(|d: &MyComplexState, _: &Env| format!("{}: {:.2}", d.term, d.scale));

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(label)
        .with_default_spacer()
        .with_child(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(searchbar)
                .with_default_spacer()
                .with_child(slider),
        )
        .center()
}
