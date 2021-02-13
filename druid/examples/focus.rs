use druid::{WindowDesc, AppLauncher, Widget, WidgetExt, Data, Lens, WidgetPod, EventCtx, LifeCycle, PaintCtx, LifeCycleCtx, BoxConstraints, LayoutCtx, Event, Env, UpdateCtx, RenderContext};
use druid::widget::{Flex, TextBox};
use piet_common::{UnitPoint, Color};
use piet_common::kurbo::{Size, Point};
use druid_shell::{HotKey, KbKey};

struct FocusWrapper<T, W: Widget<T>> {
    inner: WidgetPod<T, W>,
}

impl<T: Data, W: Widget<T>> FocusWrapper<T, W> {
    pub fn new(widget: W) -> Self {
        FocusWrapper {inner: WidgetPod::new(widget)}
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for FocusWrapper<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::KeyDown(ke) = event {
            if HotKey::new(None, KbKey::Tab).matches(ke) && ctx.is_focused() {
                ctx.focus_next();
            }
        }
        self.inner.event(ctx, event, data, env)

    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.register_for_focus();
        }

        if let LifeCycle::FocusChanged(_) = event {
            ctx.request_paint();
        }

        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(ctx, &bc.shrink((8.0, 8.0)), data, env);
        self.inner.set_origin(ctx, data, env, Point::new(4.0, 4.0));
        size + Size::new(8.0, 8.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);

        if ctx.is_focused() {
            let rect = ctx.size().to_rounded_rect(4.0);
            let brush = ctx.solid_brush(Color::AQUA);
            ctx.stroke(rect, &brush, 1.0);
        }
    }
}

#[derive(Clone, Data, Lens)]
struct AppData {
    text1: String,
    text2: String,
}

fn row() -> impl Widget<AppData> {
    Flex::row()
        .with_child(TextBox::new().lens(AppData::text1))
        .with_default_spacer()
        .with_child(TextBox::new().lens(AppData::text2))
}

fn make_widget() -> impl Widget<AppData> {
    Flex::column()
        .with_child(row())
        .with_default_spacer()
        .with_child(FocusWrapper::new(row()))
        .with_default_spacer()
        .with_child(row())
        .with_default_spacer()
        .with_child(FocusWrapper::new(row()))
        .with_default_spacer()
        .align_horizontal(UnitPoint::CENTER)
}

fn main() {
    let window = WindowDesc::new(make_widget())
        .title("Focus Test");

    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(AppData {
            text1: String::new(),
            text2: String::new(),
        })
        .expect("launch failed");
}