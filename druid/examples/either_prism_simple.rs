use druid::widget::{overlay, Button, Flex, Label};
use druid::{lens::Id, AppLauncher, Data, Lens, LocalizedString, Widget, WidgetExt, WindowDesc};

use druid::PartialPrism;

#[derive(Clone, Data, Lens)]
struct AppState {
    state: State,
}

#[derive(Clone, Data, Debug, PartialPrism)]
pub enum State {
    A(()),
    B(()),
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("prism-demo-window-title").with_placeholder("SwitcherooPrism"));
    let data = AppState {
        state: State::A(()),
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppState> {
    use druid::{optics::affine_traversal::Then, LensExt};
    Flex::column()
        .with_child(overlay::Overlay2::new(
            Label::new("a").prism((Id).guarded_by((AppState::state).then(State::a))),
            Label::new("b").prism((Id).guarded_by((AppState::state).then(State::b))),
        ))
        .with_child(
            Button::new("Switch")
                //
                .on_click(|_ctx, data: &mut AppState, _env| {
                    use State::*;
                    data.state = match data.state {
                        A(()) => B(()),
                        B(()) => A(()),
                    };
                }),
        )
}
