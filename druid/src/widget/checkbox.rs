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

//! A checkbox widget.

use crate::kurbo::{BezPath, Point, Rect, Size};
use crate::piet::{LineCap, LineJoin, LinearGradient, RenderContext, StrokeStyle, UnitPoint};
use crate::theme;
use crate::widget::{Label, LabelText};
use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, UpdateCtx,
    Widget, WidgetExt, WidgetPod,
};

/// A checkbox that toggles a `bool`.
pub struct Checkbox {
    //FIXME: this should be a TextUi struct
    child_label: WidgetPod<bool, Box<dyn Widget<bool>>>,
}

impl Checkbox {
    /// Create a new `Checkbox` with a label.
    pub fn new(label: impl Into<LabelText<bool>>) -> Checkbox {
        Checkbox {
            child_label: WidgetPod::new(Label::new(label).boxed()),
        }
    }
}

impl Widget<bool> for Checkbox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
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
                    ctx.request_paint();
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &bool, env: &Env) {
        self.child_label.lifecycle(ctx, event, data, env);
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &bool, data: &bool, env: &Env) {
        self.child_label.update(ctx, data, env);
        ctx.request_paint();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &bool, env: &Env) -> Size {
        bc.debug_check("Checkbox");

        let label_size = self.child_label.layout(ctx, &bc, data, env);
        let padding = 8.0;
        let label_x_offset = env.get(theme::BASIC_WIDGET_HEIGHT) + padding;
        let origin = Point::new(label_x_offset, 0.0);

        self.child_label.set_layout_rect(
            ctx,
            data,
            env,
            Rect::from_origin_size(origin, label_size),
        );

        bc.constrain(Size::new(
            label_x_offset + label_size.width,
            env.get(theme::BASIC_WIDGET_HEIGHT).max(label_size.height),
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, env: &Env) {
        let size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let border_width = 1.;

        let rect = Size::new(size, size)
            .to_rect()
            .inset(-border_width / 2.)
            .to_rounded_rect(2.);

        //Paint the background
        let background_gradient = LinearGradient::new(
            UnitPoint::TOP,
            UnitPoint::BOTTOM,
            (
                env.get(theme::BACKGROUND_LIGHT),
                env.get(theme::BACKGROUND_DARK),
            ),
        );

        ctx.fill(rect, &background_gradient);

        let border_color = if ctx.is_hot() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(rect, &border_color, border_width);

        if *data {
            // Paint the checkmark
            let mut path = BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));

            let mut style = StrokeStyle::new();
            style.set_line_cap(LineCap::Round);
            style.set_line_join(LineJoin::Round);

            ctx.stroke_styled(path, &env.get(theme::LABEL_COLOR), 2., &style);
        }

        // Paint the text label
        self.child_label.paint(ctx, data, env);
    }
}
