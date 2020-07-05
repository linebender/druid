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

//! Demonstrates focus and focus scope widgets

pub use druid::theme;
use druid::widget::prelude::*;
use druid::widget::{Button, Container, Flex, Focus, FocusScope, Label, TextBox, WidgetExt};

use druid::{
    commands, AppLauncher, Color, Data, LocalizedString, Point, Rect, WidgetPod, WindowDesc,
};

pub struct FocusDecorator<T> {
    pub(crate) child: WidgetPod<T, Box<dyn Widget<T>>>,
}

/// A container that draws a border around its child when focused."
impl<T: Data> FocusDecorator<T> {
    pub fn new(child: impl Widget<T> + 'static) -> Self {
        FocusDecorator {
            child: WidgetPod::new(child).boxed(),
        }
    }
}

impl<T: Data> Widget<T> for FocusDecorator<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);

        match event {
            Event::Command(cmd) if cmd.is(commands::FOCUS_NODE_FOCUS_CHANGED) => {
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, &bc, data, env);
        let rect = size.to_rect();
        self.child.set_layout_rect(ctx, data, env, rect);

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, &env);

        if ctx.focus_node().is_focused {
            let size = ctx.size();

            let rounded_rect = Rect::from_origin_size(Point::ORIGIN, size)
                .inset(-0.5)
                .to_rounded_rect(env.get(theme::BUTTON_BORDER_RADIUS));

            let border_color = env.get(theme::PRIMARY_LIGHT);

            ctx.stroke(rounded_rect, &border_color, 0.5);
        }
    }
}

fn make_column() -> impl Widget<String> {
    FocusScope::new(
        Flex::column()
            .with_child(Label::new("Focus Scope"))
            .with_default_spacer()
            .with_child(Focus::new(FocusDecorator::new(Button::new("Button 1"))))
            .with_default_spacer()
            .with_child(Focus::new(FocusDecorator::new(Button::new("Button 2"))))
            .with_default_spacer()
            .with_child(Focus::new(FocusDecorator::new(Button::new("Button 3"))))
            .with_default_spacer()
            .with_child(Focus::new(FocusDecorator::new(Button::new("Button 4"))))
            .padding(10.)
            .border(Color::grey(0.6), 2.0)
            .rounded(5.0),
    )
}

fn root_scope_child() -> impl Widget<String> {
    Flex::row()
        .with_default_spacer()
        .with_child(Focus::new(FocusDecorator::new(Button::new("Root"))).with_auto_focus(true))
        .with_default_spacer()
        .with_child(TextBox::new().with_placeholder("Test"))
        .with_default_spacer()
        .with_child(Container::new(make_column()))
        .with_default_spacer()
        .with_child(Container::new(make_column()))
        .with_default_spacer()
        .with_child(Focus::new(FocusDecorator::new(Button::new("Root"))))
        .with_default_spacer()
        .with_child(Focus::new(FocusDecorator::new(Button::new("Root"))))
        .with_default_spacer()
        .center()
        .padding(10.)
}

fn make_ui() -> impl Widget<String> {
    Flex::column()
        .with_default_spacer()
        .with_child(Label::new("Focus Scope"))
        .with_default_spacer()
        .with_flex_spacer(1.)
        .with_child(root_scope_child())
        .with_flex_spacer(1.)
        .border(Color::grey(0.6), 2.0)
        .rounded(5.0)
        .padding(10.)
}

pub fn main() {
    let main_window = WindowDesc::new(make_ui)
        .window_size((620., 600.00))
        .with_min_size((620., 265.00))
        .title(LocalizedString::new("Focus; FocusScope"));

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(String::new())
        .expect("launch failed");
}
