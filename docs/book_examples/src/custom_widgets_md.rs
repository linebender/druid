use druid::widget::{Controller, Label, Painter, SizedBox, TextBox};
use druid::{Color, Env, Event, EventCtx, KeyCode, PaintCtx, RenderContext, Widget, WidgetExt};

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
        let (r, g, b, _) = color.as_rgba_u8();
        format!("#{:X}{:X}{:X}", r, g, b)
    })
    .background(make_color_swatch())
}
// ANCHOR_END: background_label

// ANCHOR: annoying_textbox
#[derive(Default)]
struct AnnoyingController {
    suppress_next: bool,
}

impl Controller<String, TextBox> for AnnoyingController {
    fn event(
        &mut self,
        child: &mut TextBox,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut String,
        env: &Env,
    ) {
        if matches!(event, Event::KeyDown(k) if k.key_code == KeyCode::Backspace) {
            self.suppress_next = !self.suppress_next;
            if self.suppress_next {
                return;
            }
        }

        // if we want our child to receive this event, we must send it explicitly.
        child.event(ctx, event, data, env);
    }
}
// ANCHOR_END: annoying_textbox
