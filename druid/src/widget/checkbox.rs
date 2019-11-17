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

//! A checkbox widget.

use crate::kurbo::{BezPath, Point, RoundedRect, Size};
use crate::piet::{LineCap, LineJoin, LinearGradient, RenderContext, StrokeStyle, UnitPoint};
use crate::theme;
use crate::widget::Align;
use crate::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

/// A checkbox that toggles a boolean
#[derive(Debug, Clone, Default)]
pub struct Checkbox;

impl Checkbox {
    pub fn new() -> impl Widget<bool> {
        Align::vertical(UnitPoint::CENTER, Self::default())
    }
}

impl Widget<bool> for Checkbox {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, base_state: &BaseState, data: &bool, env: &Env) {
        let size = env.get(theme::BASIC_WIDGET_HEIGHT);

        let rect =
            RoundedRect::from_origin_size(Point::ORIGIN, Size::new(size, size).to_vec2(), 2.);

        //Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );

        paint_ctx.fill(rect, &background_gradient);

        let border_color = if base_state.is_hot() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER)
        };

        paint_ctx.stroke(rect, &border_color, 1.);

        if *data {
            let mut path = BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));

            let mut style = StrokeStyle::new();
            style.set_line_cap(LineCap::Round);
            style.set_line_join(LineJoin::Round);

            paint_ctx.stroke_styled(path, &env.get(theme::LABEL_COLOR), 2., &style);
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &bool,
        env: &Env,
    ) -> Size {
        bc.debug_check("Checkbox");

        bc.constrain(Size::new(
            env.get(theme::BASIC_WIDGET_HEIGHT),
            env.get(theme::BASIC_WIDGET_HEIGHT),
        ))
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        if *data {
                            *data = false;
                        } else {
                            *data = true;
                        }
                    }
                    ctx.invalidate();
                }
            }
            Event::HotChanged(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: Option<&bool>, _data: &bool, _env: &Env) {
        ctx.invalidate();
    }
}
