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

use crate::kurbo::{BezPath, Size};
use crate::piet::{LineCap, LineJoin, LinearGradient, RenderContext, StrokeStyle, UnitPoint};
use crate::theme;
use crate::widget::{prelude::*, Label, LabelText};

/// A checkbox that toggles a `bool`.
pub struct Checkbox {
    child_label: Label<bool>,
}

impl Checkbox {
    /// Create a new `Checkbox` with a text label.
    pub fn new(text: impl Into<LabelText<bool>>) -> Checkbox {
        Checkbox {
            child_label: Label::new(text),
        }
    }

    /// Update the text label.
    pub fn set_text(&mut self, label: impl Into<LabelText<bool>>) {
        self.child_label.set_text(label);
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

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, env: &Env) {
        self.child_label.update(ctx, old_data, data, env);
        ctx.request_paint();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &bool, env: &Env) -> Size {
        bc.debug_check("Checkbox");
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);
        let check_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let label_size = self.child_label.layout(ctx, &bc, data, env);

        let desired_size = Size::new(
            check_size + x_padding + label_size.width,
            check_size.max(label_size.height),
        );
        let our_size = bc.constrain(desired_size);
        let baseline = self.child_label.baseline_offset() + (our_size.height - label_size.height);
        ctx.set_baseline_offset(baseline);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, env: &Env) {
        let size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);
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

            let style = StrokeStyle::new()
                .line_cap(LineCap::Round)
                .line_join(LineJoin::Round);

            ctx.stroke_styled(path, &env.get(theme::LABEL_COLOR), 2., &style);
        }

        // Paint the text label
        self.child_label.draw_at(ctx, (size + x_padding, 0.0));
    }
}
