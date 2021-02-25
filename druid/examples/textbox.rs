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

//! An example of various text layout features.

use std::sync::Arc;

use druid::widget::prelude::*;
use druid::widget::{Checkbox, Controller, Flex, ImeTextBox, Label, RadioGroup};
use druid::{
    AppLauncher, Color, Data, Lens, LensExt, LocalizedString, Selector, TextAlignment, Widget,
    WidgetExt, WindowDesc,
};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Text Options");

#[derive(Clone, Data, Lens)]
struct AppState {
    text: Arc<String>,
    options: Options,
}

#[derive(Debug, Clone, Default, Data, Lens)]
struct Options {
    //line_break_mode: LineBreaking,
    alignment: TextAlignment,
    line_wrap: bool,
    multiline: bool,
}

const UPDATE_OPTIONS: Selector<Options> = Selector::new("druid-example.textbox-update-options");

struct OptionSender;
impl<W: Widget<AppState>> Controller<AppState, W> for OptionSender {
    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if !old_data.options.same(&data.options) {
            ctx.submit_command(UPDATE_OPTIONS.with(data.options.clone()));
        }
        child.update(ctx, old_data, data, env);
    }
}

/// A controller that updates label properties as required.
struct OptionUpdater;

impl Controller<Arc<String>, ImeTextBox<Arc<String>>> for OptionUpdater {
    fn event(
        &mut self,
        child: &mut ImeTextBox<Arc<String>>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Arc<String>,
        env: &Env,
    ) {
        if let Event::Command(cmd) = event {
            if let Some(options) = cmd.get(UPDATE_OPTIONS) {
                *child = make_text_box(options);
                ctx.children_changed();
                if !options.multiline {
                    let first_line = data.lines().next().to_owned().unwrap_or_default();
                    *Arc::make_mut(data) = first_line.into();
                }
                ctx.request_layout();
                ctx.set_handled();
                return;
            }
        }
        child.event(ctx, event, data, env)
    }
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .window_size((400.0, 600.0));

    // create the initial app state
    let initial_state = AppState {
        text: "".to_string().into(),
        options: Default::default(),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn make_text_box(options: &Options) -> ImeTextBox<Arc<String>> {
    dbg!(options);
    if options.multiline {
        ImeTextBox::multiline().with_line_wrapping(options.line_wrap)
    } else {
        ImeTextBox::new()
    }
    .with_text_alignment(options.alignment)
    .with_placeholder("Toy TextBox, much 2 big")
    .with_text_size(16.0)
}

fn build_root_widget() -> impl Widget<AppState> {
    let textbox = Flex::row()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
        .with_child(Label::new("TextBox:"))
        .with_default_spacer()
        .with_flex_child(
            make_text_box(&Options::default())
                .controller(OptionUpdater)
                .lens(AppState::text),
            1.0,
        );

    let alignment_picker = Flex::column()
        .with_child(Label::new("Justification"))
        .with_default_spacer()
        .with_child(RadioGroup::new(vec![
            ("Start", TextAlignment::Start),
            ("End", TextAlignment::End),
            ("Center", TextAlignment::Center),
            ("Justified", TextAlignment::Justified),
        ]))
        .lens(AppState::options.then(Options::alignment));

    let multiline_picker = Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(Label::new("Mutiline"))
        .with_default_spacer()
        .with_child(
            Checkbox::new("Allow multiple lines").lens(AppState::options.then(Options::multiline)),
        )
        .with_default_spacer()
        .with_child(
            Checkbox::new("Soft wrap lines").lens(AppState::options.then(Options::line_wrap)),
        );

    let controls = Flex::row()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(alignment_picker)
        .with_default_spacer()
        .with_child(multiline_picker)
        .padding(8.0)
        .border(Color::grey(0.6), 2.0)
        .rounded(5.0);

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(controls)
        .with_spacer(24.0)
        .with_flex_child(textbox, 1.0)
        .padding(8.0)
        .controller(OptionSender)
    //.debug_paint_layout()
}
