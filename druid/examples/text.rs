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

use druid::widget::{Controller, Flex, Label, LineBreaking, RadioGroup, Scroll};
use druid::{
    AppLauncher, Color, Data, Env, Lens, LocalizedString, TextAlignment, UpdateCtx, Widget,
    WidgetExt, WindowDesc,
};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Text Options");

const TEXT: &str = r#"Contrary to what we would like to believe, there is no such thing as a structureless group. Any group of people of whatever nature that comes together for any length of time for any purpose will inevitably structure itself in some fashion. The structure may be flexible; it may vary over time; it may evenly or unevenly distribute tasks, power and resources over the members of the group. But it will be formed regardless of the abilities, personalities, or intentions of the people involved. The very fact that we are individuals, with different talents, predispositions, and backgrounds makes this inevitable. Only if we refused to relate or interact on any basis whatsoever could we approximate structurelessness -- and that is not the nature of a human group.
This means that to strive for a structureless group is as useful, and as deceptive, as to aim at an "objective" news story, "value-free" social science, or a "free" economy. A "laissez faire" group is about as realistic as a "laissez faire" society; the idea becomes a smokescreen for the strong or the lucky to establish unquestioned hegemony over others. This hegemony can be so easily established because the idea of "structurelessness" does not prevent the formation of informal structures, only formal ones. Similarly "laissez faire" philosophy did not prevent the economically powerful from establishing control over wages, prices, and distribution of goods; it only prevented the government from doing so. Thus structurelessness becomes a way of masking power, and within the women's movement is usually most strongly advocated by those who are the most powerful (whether they are conscious of their power or not). As long as the structure of the group is informal, the rules of how decisions are made are known only to a few and awareness of power is limited to those who know the rules. Those who do not know the rules and are not chosen for initiation must remain in confusion, or suffer from paranoid delusions that something is happening of which they are not quite aware."#;

const SPACER_SIZE: f64 = 8.0;

#[derive(Clone, Data, Lens)]
struct AppState {
    line_break_mode: LineBreaking,
    alignment: TextAlignment,
}

/// A controller that sets properties on a label.
struct LabelController;

impl Controller<AppState, Label<AppState>> for LabelController {
    #[allow(clippy::float_cmp)]
    fn update(
        &mut self,
        child: &mut Label<AppState>,
        ctx: &mut UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if old_data.line_break_mode != data.line_break_mode {
            child.set_line_break_mode(data.line_break_mode);
            ctx.request_layout();
        }
        if old_data.alignment != data.alignment {
            child.set_text_alignment(data.alignment);
        }
        child.update(ctx, old_data, data, env);
    }
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title(WINDOW_TITLE)
        .window_size((400.0, 600.0));

    // create the initial app state
    let initial_state = AppState {
        line_break_mode: LineBreaking::Clip,
        alignment: Default::default(),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    let label = Scroll::new(
        Label::new(TEXT)
            .with_text_color(Color::BLACK)
            .controller(LabelController)
            .background(Color::WHITE)
            .expand_width()
            .padding((SPACER_SIZE * 4.0, SPACER_SIZE))
            .background(Color::grey8(222)),
    )
    .vertical();

    let line_break_chooser = Flex::column()
        .with_child(Label::new("Line break mode"))
        .with_spacer(SPACER_SIZE)
        .with_child(RadioGroup::new(vec![
            ("Clip", LineBreaking::Clip),
            ("Wrap", LineBreaking::WordWrap),
            ("Overflow", LineBreaking::Overflow),
        ]))
        .lens(AppState::line_break_mode);

    let alignment_picker = Flex::column()
        .with_child(Label::new("Justification"))
        .with_spacer(SPACER_SIZE)
        .with_child(RadioGroup::new(vec![
            ("Start", TextAlignment::Start),
            ("End", TextAlignment::End),
            ("Center", TextAlignment::Center),
            ("Justified", TextAlignment::Justified),
        ]))
        .lens(AppState::alignment);

    let controls = Flex::row()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(alignment_picker)
        .with_spacer(SPACER_SIZE)
        .with_child(line_break_chooser)
        .padding(SPACER_SIZE);

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(controls)
        .with_flex_child(label, 1.0)
}
