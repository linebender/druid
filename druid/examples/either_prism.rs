use druid::widget::{overlay, Checkbox, Flex, Label, Slider};
use druid::{AppLauncher, Data, Lens, LocalizedString, Widget, WidgetExt, WindowDesc};

use druid::PartialPrism;
use druid::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, UpdateCtx,
};

#[derive(Clone, Default, Data, Lens)]
struct AppState {
    value: f64,
    panel: SliderOrLabel,
}

#[derive(Clone, Data, Debug, PartialPrism)]
pub enum SliderOrLabel {
    Slider {
        // TODO
        save_value: bool,
    },
    Label(String),
}

impl From<bool> for SliderOrLabel {
    fn from(b: bool) -> Self {
        match b {
            false => Self::Slider { save_value: false },
            true => Self::Label("(From): Click to reveal slider".into()),
        }
    }
}

impl From<&SliderOrLabel> for bool {
    fn from(sol: &SliderOrLabel) -> Self {
        match &sol {
            SliderOrLabel::Slider { .. } => false,
            SliderOrLabel::Label(_) => true,
        }
    }
}

impl Default for SliderOrLabel {
    fn default() -> Self {
        Self::Label("(Default): Click to reveal slider".into())
        // Self::Slider { save_value: false }
    }
}

impl Widget<SliderOrLabel> for Checkbox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut SliderOrLabel, env: &Env) {
        let data_before: bool = (&*data).into();
        let mut data_after: bool = data_before;
        Widget::<bool>::event(self, ctx, event, &mut data_after, env);
        if data_before != data_after {
            let data_after = SliderOrLabel::from(data_after);
            *data = data_after;
        }
    }
    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &SliderOrLabel,
        env: &Env,
    ) {
        Widget::<bool>::lifecycle(self, ctx, event, &data.into(), env)
    }
    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &SliderOrLabel,
        data: &SliderOrLabel,
        env: &Env,
    ) {
        let old_data: bool = (&*old_data).into();
        let data: bool = (&*data).into();
        Widget::<bool>::update(self, ctx, &old_data, &data, env)
    }
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &SliderOrLabel,
        env: &Env,
    ) -> druid::Size {
        let data: bool = (&*data).into();
        Widget::<bool>::layout(self, ctx, bc, &data, env)
    }
    fn paint(&mut self, ctx: &mut PaintCtx, data: &SliderOrLabel, env: &Env) {
        let data: bool = (&*data).into();
        Widget::<bool>::paint(self, ctx, &data, env)
    }
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("prism-demo-window-title").with_placeholder("SwitcherooPrism"));
    let data = AppState::default();
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppState> {
    use druid::optics::affine_traversal::Then;
    use druid::LensExt;

    let mut col = Flex::column();
    col.add_child(
        Checkbox::new("Toggle slider")
            .lens(AppState::panel)
            .padding(5.0),
    );
    let panel_slider = Slider::new()
        .prism((AppState::value).guarded_by((AppState::panel).then(SliderOrLabel::slider)))
        .padding(5.0);
    let panel_label = label()
        .padding(5.0)
        .prism((AppState::panel).then(SliderOrLabel::label));
    let panel = overlay::Overlay2::new(panel_slider, panel_label);
    col.add_child(panel);
    col
}

fn label() -> impl Widget<String> {
    Label::new(|data: &String, _env: &_| data.to_string())
}
