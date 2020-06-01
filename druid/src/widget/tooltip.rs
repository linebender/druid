use std::time::{Duration, Instant};

use crate::widget::prelude::*;
use crate::widget::{Controller, Label, LabelText, WidgetExt};
use crate::{Color, Data, ModalDesc, Point, TimerToken};

// TODO: put in env
const TOOLTIP_DELAY: Duration = Duration::from_millis(500);
const TOOLTIP_BORDER_WIDTH: f64 = 1.0;
const TOOLTIP_BORDER_COLOR: Color = Color::WHITE;
const TOOLTIP_BACKGROUND_COLOR: Color = Color::BLACK;
const TOOLTIP_TEXT_COLOR: Color = Color::WHITE;

/// A controller that listens for mouse hovers and displays a tooltip in response.
pub struct TooltipWrap<T> {
    text: LabelText<T>,
    timer: TimerToken,
    // If we are considering showing a tooltip, this will be the time of the last
    // mouse move event.
    last_mouse_move: Option<Instant>,
    mouse_pos: Point,
}

/// The tooltip widgets get wrapped by this controller, which dismisses the tooltip on any user
/// input.
struct TooltipController;

impl<W: Widget<()>> Controller<(), W> for TooltipController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut (),
        env: &Env,
    ) {
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

        child.event(ctx, event, data, env);
    }
}

fn tooltip_desc(text: &str, position: Point) -> ModalDesc<()> {
    ModalDesc::new(
        Label::new(text)
            .with_text_color(TOOLTIP_TEXT_COLOR)
            .border(TOOLTIP_BORDER_COLOR, TOOLTIP_BORDER_WIDTH)
            .background(TOOLTIP_BACKGROUND_COLOR)
            .controller(TooltipController),
    )
    .pass_through_events(true)
    .position(position)
}

impl<T: Data> TooltipWrap<T> {
    pub(crate) fn new(text: LabelText<T>) -> TooltipWrap<T> {
        TooltipWrap {
            text,
            timer: TimerToken::INVALID,
            last_mouse_move: None,
            mouse_pos: Point::ZERO,
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for TooltipWrap<T> {
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
                        .unwrap_or(Duration::from_millis(0));
                    if elapsed > check_delay {
                        self.text.resolve(data, env);
                        ctx.show_static_modal(tooltip_desc(
                            &self.text.display_text(),
                            self.mouse_pos,
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
