use druid::keyboard_types::Key;
use druid::widget::{Controller, Label, Painter, SizedBox, TextBox};
use druid::{
    Color, Env, Event, EventCtx, PaintCtx, RenderContext, Selector, TimerToken, Widget, WidgetExt,
};
use std::time::Duration;

const CORNER_RADIUS: f64 = 4.0;
const STROKE_WIDTH: f64 = 2.0;

// ANCHOR: color_swatch
fn make_color_swatch() -> Painter<Color> {
    Painter::new(|ctx: &mut PaintCtx, data: &Color, env: &Env| {
        let bounds = ctx.size().to_rect();
        let rounded = bounds.to_rounded_rect(CORNER_RADIUS);
        ctx.fill(rounded, data);
        ctx.stroke(rounded, &env.get(druid::theme::PRIMARY_DARK), STROKE_WIDTH);
    })
}
// ANCHOR_END: color_swatch

// ANCHOR: sized_swatch
fn sized_swatch() -> impl Widget<Color> {
    SizedBox::new(make_color_swatch()).width(20.0).height(20.0)
}
// ANCHOR_END: sized_swatch

// ANCHOR: background_label
fn background_label() -> impl Widget<Color> {
    Label::dynamic(|color: &Color, _| {
        let (r, g, b, _) = color.as_rgba8();
        format!("#{:X}{:X}{:X}", r, g, b)
    })
    .background(make_color_swatch())
}
// ANCHOR_END: background_label

// ANCHOR: annoying_textbox
const ACTION: Selector = Selector::new("hello.textbox-action");
const DELAY: Duration = Duration::from_millis(300);

struct TextBoxActionController {
    timer: Option<TimerToken>,
}

impl TextBoxActionController {
    pub fn new() -> Self {
        TextBoxActionController { timer: None }
    }
}

impl Controller<String, TextBox> for TextBoxActionController {
    fn event(
        &mut self,
        child: &mut TextBox,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        match event {
            Event::KeyDown(k) if k.key == Key::Enter => {
                ctx.submit_command(ACTION);
            }
            Event::KeyUp(k) if k.key == Key::Enter => {
                self.timer = Some(ctx.request_timer(DELAY));
                child.event(ctx, event, data, env);
            }
            Event::Timer(token) if Some(*token) == self.timer => {
                ctx.submit_command(ACTION);
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
// ANCHOR_END: annoying_textbox
