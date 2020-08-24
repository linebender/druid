use druid::widget::Slider;
use druid::widget::{CrossAxisAlignment, Flex, Label, Switch, TextBox};
use druid::{AppLauncher, Data, Env, LocalizedString, Prism, Widget, WidgetExt, WindowDesc};

pub fn main() {
    let main_window = WindowDesc::new(ui_builder)
        .title(LocalizedString::new("lens-demo-window-title").with_placeholder("Lens Demo"));
    let data = State::Term("hello".into());

    AppLauncher::with_window(main_window)
        .launch(data)
        .expect("launch failed");
}

#[derive(Clone, Debug, Data, Prism)]
pub enum State {
    Term(String),
    Scale(f64),
}

fn ui_builder() -> impl Widget<State> {
    let searchbar = TextBox::new().prism(State::term);
    let slider = Slider::new().prism(State::scale);
    let label = Label::new(|d: &State, _: &Env| format!("{:?}", d));

    let switch = Switch::new().on_click(|_evt, _bool, _env| {
        //
        todo!()
    });

    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .with_child(label)
        .with_spacer(8.0)
        .with_child(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(searchbar)
                .with_spacer(8.0)
                .with_child(slider),
        )
        .center()
}
