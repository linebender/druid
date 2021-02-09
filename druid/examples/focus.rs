use druid::{WindowDesc, AppLauncher, Widget, WidgetExt, Data, Lens};
use druid::widget::{Flex, TextBox};
use piet_common::UnitPoint;

#[derive(Clone, Data, Lens)]
struct AppData {
    text1: String,
}

fn row() -> impl Widget<AppData> {
    TextBox::new().lens(AppData::text1)
}

fn make_widget() -> impl Widget<AppData> {
    Flex::column()
        .with_child(row())
        .with_default_spacer()
        .with_child(row())
        .with_default_spacer()
        .with_child(row())
        .with_default_spacer()
        .with_child(row())
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
        })
        .expect("launch failed");
}