// Copyright 2020 The xi-editor Authors.
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

use std::time::Duration;

use instant::Instant;

use crate::widget::prelude::*;
use crate::widget::{Controller, Label, LabelText, WidgetExt};
use crate::{Color, Data, ModalDesc, Point, Rect, TimerToken, Vec2, WidgetPod};

// TODO: put in env
const TOOLTIP_DELAY: Duration = Duration::from_millis(500);
const TOOLTIP_BORDER_WIDTH: f64 = 1.0;
const TOOLTIP_BORDER_COLOR: Color = Color::WHITE;
const TOOLTIP_BACKGROUND_COLOR: Color = Color::BLACK;
const TOOLTIP_TEXT_COLOR: Color = Color::WHITE;
const TOOLTIP_TEXT_PADDING: f64 = 3.0;

// We don't want to draw the tooltip *right* on the mouse, because it'll be in the way.
const TOOLTIP_OFFSET: Vec2 = Vec2::new(10.0, 10.0);

/// A controller that listens for mouse hovers and displays a tooltip in response.
///
/// See [`WidgetExt::tooltip`] for a nicer interface to this functionality.
///
/// [`WidgetExt::tooltip`]: ../trait.WidgetExt.html#method.tooltip
pub struct TooltipController<T> {
    text: LabelText<T>,
    timer: TimerToken,
    // The naive implementation of a tooltip timer would be: on every mouse move, create a new
    // timer, but only keep the token of the last one. When the last one fires, show the tooltip.
    //
    // We do something a little more complicated, to avoid setting so many timers: if we get a
    // mouse move while a timer's already running, we update `last_mouse_move` instead of starting
    // a new timer. That way, when the timer fires we know how long it's been since the last mouse
    // move.
    last_mouse_move: Option<Instant>,
    mouse_pos: Point,
}

/// The tooltip widgets get wrapped by this overlay, which dismisses the tooltip on any user
/// input.
///
/// The overlay is a widget, rather than just a controller, because we want to tweak it's layout
/// to fill its entire parent (so that we don't miss events).
struct TooltipOverlay<W> {
    tooltip_origin: Point,
    tooltip: WidgetPod<(), W>,
}

impl<W: Widget<()>> Widget<()> for TooltipOverlay<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut (), env: &Env) {
        match event {
            Event::MouseDown(_)
            | Event::MouseUp(_)
            | Event::MouseMove(_)
            | Event::Wheel(_)
            | Event::KeyDown(_)
            | Event::KeyUp(_) => {
                ctx.dismiss_modal();
            }
            _ => {}
        }

        self.tooltip.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &(), env: &Env) {
        self.tooltip.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &(), data: &(), env: &Env) {
        self.tooltip.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &(), env: &Env) -> Size {
        let tooltip_size = self.tooltip.layout(ctx, bc, data, env);
        let size = bc.max();

        // If the tooltip would extend outside our bounds, try to make it fit.
        let tooltip_x = self
            .tooltip_origin
            .x
            .min(size.width - tooltip_size.width)
            .max(0.0);
        let tooltip_y = self
            .tooltip_origin
            .y
            .min(size.height - tooltip_size.height)
            .max(0.0);
        let tooltip_origin = Point::new(tooltip_x, tooltip_y);
        self.tooltip.set_layout_rect(
            ctx,
            data,
            env,
            Rect::from_origin_size(tooltip_origin, tooltip_size),
        );

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &(), env: &Env) {
        self.tooltip.paint(ctx, data, env);
    }
}

fn tooltip_desc(text: &str, position: Point) -> ModalDesc<()> {
    let tooltip = Label::new(text)
        .with_text_color(TOOLTIP_TEXT_COLOR)
        .padding(TOOLTIP_TEXT_PADDING)
        .border(TOOLTIP_BORDER_COLOR, TOOLTIP_BORDER_WIDTH)
        .background(TOOLTIP_BACKGROUND_COLOR);
    ModalDesc::new(TooltipOverlay {
        tooltip: WidgetPod::new(tooltip),
        tooltip_origin: position,
    })
    .pass_through_events(true)
    .position(Point::ZERO)
}

impl<T: Data> TooltipController<T> {
    pub(crate) fn new(text: LabelText<T>) -> TooltipController<T> {
        TooltipController {
            text,
            timer: TimerToken::INVALID,
            last_mouse_move: None,
            mouse_pos: Point::ZERO,
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for TooltipController<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, ev: &Event, data: &mut T, env: &Env) {
        match ev {
            Event::MouseDown(_) | Event::MouseUp(_) => {
                self.timer = TimerToken::INVALID;
                self.last_mouse_move = None;
            }
            Event::MouseMove(ev) => {
                self.last_mouse_move = if ctx.is_hot() {
                    if self.timer == TimerToken::INVALID {
                        self.timer = ctx.request_timer(TOOLTIP_DELAY);
                    }
                    self.mouse_pos = ev.window_pos;
                    Some(Instant::now())
                } else {
                    None
                };
            }
            Event::Timer(tok) if tok == &self.timer => {
                self.timer = TimerToken::INVALID;
                if let Some(move_time) = self.last_mouse_move {
                    let elapsed = Instant::now().duration_since(move_time);
                    // Check whether the required time has elapsed. We allow a little slack to
                    // account for not-completely-accurate clocks.
                    let check_delay = TOOLTIP_DELAY
                        .checked_sub(Duration::from_millis(20))
                        .unwrap_or_else(|| Duration::from_millis(0));
                    if elapsed > check_delay {
                        self.text.resolve(data, env);
                        ctx.show_static_modal(tooltip_desc(
                            &self.text.display_text(),
                            self.mouse_pos + TOOLTIP_OFFSET,
                        ));
                        self.timer = TimerToken::INVALID;
                        self.last_mouse_move = None;
                    } else {
                        self.timer = ctx.request_timer(TOOLTIP_DELAY - elapsed);
                    }
                }
            }
            _ => {}
        }
        child.event(ctx, ev, data, env);
    }
}
