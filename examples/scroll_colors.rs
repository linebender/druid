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

//! This example allows to play with scroll bars over different color tones.

use druid::shell::kurbo::{Rect, Size};
use druid::shell::piet::{Color, RenderContext};
use druid::widget::{Column, Row, Scroll};
use druid::{
    AppLauncher, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx,
    UpdateCtx, Widget, WindowDesc,
};

struct FixedSize {
    size: Size,
    color: Color,
}

impl FixedSize {
    pub fn new(size: Size, color: Color) -> Self {
        Self { size, color }
    }
}

impl<T: Data> Widget<T> for FixedSize {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, _data: &T, _env: &Env) {
        let color = if base_state.is_hot() {
            Color::BLACK
        } else {
            self.color.clone()
        };
        let brush = paint_ctx.render_ctx.solid_brush(color);
        let rect = Rect::from_origin_size((0.0, 0.0), base_state.size());
        paint_ctx.render_ctx.fill(rect, &brush);
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        self.size.clone()
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, _data: &mut T, _env: &Env) {
        match event {
            Event::HotChanged(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: Option<&T>, _data: &T, _env: &Env) {}
}

fn build_app() -> impl Widget<u32> {
    let mut col = Column::new();
    let rows = 30;
    let cols = 30;

    for i in 0..cols {
        let mut row = Row::new();
        let col_progress = i as f64 / cols as f64;

        for j in 0..rows {
            let row_progress = j as f64 / rows as f64;

            row.add_child(
                // There is a SizedBox widget but it's not currently possible
                // to set a background on it.
                FixedSize::new(
                    Size::new(50.0, 50.0),
                    Color::rgb(1.0 * col_progress, 1.0 * row_progress, 1.0),
                ),
                0.0,
            );
        }

        col.add_child(row, 0.0);
    }

    Scroll::new(col)
}

fn main() {
    simple_logger::init().unwrap();
    let main_window = WindowDesc::new(build_app);
    let data = 0_u32;
    AppLauncher::with_window(main_window)
        .launch(data)
        .expect("launch failed");
}
