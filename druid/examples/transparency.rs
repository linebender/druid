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

//! An example of a transparent window background.
//! Useful for dropdowns, tooltips and other overlay windows.

use druid::kurbo::Circle;
use druid::widget::prelude::*;
use druid::widget::{Button, Flex};
use druid::{AppLauncher, Color, Rect, WindowDesc};

struct CustomWidget;

impl Widget<String> for CustomWidget {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut String, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &String,
        _env: &Env,
    ) {
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &String, _data: &String, _env: &Env) {}

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &String,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    // The paint method gets called last, after an event flow.
    // It goes event -> update -> layout -> paint, and each method can influence the next.
    // Basically, anything that changes the appearance of a widget causes a paint.
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &String, _env: &Env) {
        let boundaries = ctx.size().to_rect();
        let center = (boundaries.width() / 2., boundaries.height() / 2.);
        let circle = Circle::new(center, center.0.min(center.1));
        ctx.fill(circle, &Color::RED);

        let rect1 = Rect::new(0., 0., boundaries.width() / 2., boundaries.height() / 2.);
        ctx.fill(rect1, &Color::rgba8(0x0, 0xff, 0, 125));

        let rect2 = Rect::new(
            boundaries.width() / 2.,
            boundaries.height() / 2.,
            boundaries.width(),
            boundaries.height(),
        );
        ctx.fill(rect2, &Color::rgba8(0x0, 0x0, 0xff, 125));
    }
}

pub fn main() {
    let btn = Button::new("Example button on transparent bg");
    let example = Flex::column()
        .with_flex_child(CustomWidget {}, 10.0)
        .with_flex_child(btn, 1.0);
    let window = WindowDesc::new(example)
        .show_titlebar(false)
        .set_position((50., 50.))
        .window_size((823., 823.))
        .transparent(true)
        .resizable(true)
        .title("Transparent background");

    AppLauncher::with_window(window)
        .use_env_tracing()
        .launch("Druid + Piet".to_string())
        .expect("launch failed");
}
