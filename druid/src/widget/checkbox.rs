// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A checkbox widget.

use crate::debug_state::DebugState;
use crate::kurbo::{BezPath, Size};
use crate::piet::{LineCap, LineJoin, LinearGradient, RenderContext, StrokeStyle, UnitPoint};
use crate::theme;
use crate::widget::{prelude::*, Label, LabelText};
use tracing::{instrument, trace};

/// A checkbox that toggles a `bool`.
pub struct Checkbox {
    child_label: Label<bool>,
}

impl Checkbox {
    /// Create a new `Checkbox` with a text label.
    pub fn new(text: impl Into<LabelText<bool>>) -> Checkbox {
        Self::from_label(Label::new(text))
    }

    /// Create a new `Checkbox` with the provided [`Label`].
    pub fn from_label(label: Label<bool>) -> Checkbox {
        Checkbox { child_label: label }
    }

    /// Update the text label.
    pub fn set_text(&mut self, label: impl Into<LabelText<bool>>) {
        self.child_label.set_text(label);
    }
}

impl Widget<bool> for Checkbox {
    #[instrument(name = "CheckBox", level = "trace", skip(self, ctx, event, data, _env))]
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                if !ctx.is_disabled() {
                    ctx.set_active(true);
                    ctx.request_paint();
                    trace!("Checkbox {:?} pressed", ctx.widget_id());
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() && !ctx.is_disabled() {
                    if ctx.is_hot() {
                        if *data {
                            *data = false;
                            trace!("Checkbox {:?} released - unchecked", ctx.widget_id());
                        } else {
                            *data = true;
                            trace!("Checkbox {:?} released - checked", ctx.widget_id());
                        }
                    }
                    ctx.request_paint();
                }
                ctx.set_active(false);
            }
            _ => (),
        }
    }

    #[instrument(name = "CheckBox", level = "trace", skip(self, ctx, event, data, env))]
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &bool, env: &Env) {
        self.child_label.lifecycle(ctx, event, data, env);
        if let LifeCycle::HotChanged(_) | LifeCycle::DisabledChanged(_) = event {
            ctx.request_paint();
        }
    }

    #[instrument(
        name = "CheckBox",
        level = "trace",
        skip(self, ctx, old_data, data, env)
    )]
    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, env: &Env) {
        self.child_label.update(ctx, old_data, data, env);
        ctx.request_paint();
    }

    #[instrument(name = "CheckBox", level = "trace", skip(self, ctx, bc, data, env))]
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &bool, env: &Env) -> Size {
        bc.debug_check("Checkbox");
        let x_padding = env.get(theme::WIDGET_CONTROL_COMPONENT_PADDING);
        let check_size = env.get(theme::BASIC_WIDGET_HEIGHT);
        let label_size = self.child_label.layout(ctx, bc, data, env);

        let desired_size = Size::new(
            check_size + x_padding + label_size.width,
            check_size.max(label_size.height),
        );
        let our_size = bc.constrain(desired_size);
        let baseline = self.child_label.baseline_offset() + (our_size.height - label_size.height);
        ctx.set_baseline_offset(baseline);
        trace!("Computed layout: size={}, baseline={}", our_size, baseline);
        our_size
    }

    #[instrument(name = "CheckBox", level = "trace", skip(self, ctx, data, env))]
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

        let border_color = if ctx.is_hot() && !ctx.is_disabled() {
            env.get(theme::BORDER_LIGHT)
        } else {
            env.get(theme::BORDER_DARK)
        };

        ctx.stroke(rect, &border_color, border_width);

        if *data {
            // Paint the checkmark
            let x_offset = (rect.width() - 10.0) / 2.0;
            let y_offset = (rect.height() - 8.0) / 2.0;
            let mut path = BezPath::new();
            path.move_to((x_offset, y_offset + 4.0));
            path.line_to((x_offset + 4.0, y_offset + 8.0));
            path.line_to((x_offset + 10.0, y_offset));

            let style = StrokeStyle::new()
                .line_cap(LineCap::Round)
                .line_join(LineJoin::Round);

            let brush = if ctx.is_disabled() {
                env.get(theme::DISABLED_TEXT_COLOR)
            } else {
                env.get(theme::TEXT_COLOR)
            };

            ctx.stroke_styled(path, &brush, 2., &style);
        }

        // Paint the text label
        self.child_label.draw_at(ctx, (size + x_padding, 0.0));
    }

    fn debug_state(&self, data: &bool) -> DebugState {
        let display_value = if *data {
            format!("[X] {}", self.child_label.text())
        } else {
            format!("[_] {}", self.child_label.text())
        };
        DebugState {
            display_name: self.short_type_name().to_string(),
            main_value: display_value,
            ..Default::default()
        }
    }
}
