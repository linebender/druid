use druid::{AppLauncher, Data, Widget, WindowDesc};
use druid::widget::{Button, Flex, Label};

#[derive(Debug, Clone, Data)]
struct ExampleData {
    is_decorated: bool,
    has_titlebar: bool,
    is_resizable: bool,
    is_transparent: bool,
}

pub fn main() {
    let data = ExampleData {
        is_decorated: true,
        has_titlebar: true,
        is_resizable: true,
        is_transparent: false, //Can only be done when the window is created.
    };

    let window = WindowDesc::new(build_root_widget())
        .show_decorations(data.is_decorated)
        .show_titlebar(data.has_titlebar)
        .resizable(data.is_resizable)
        .transparent(data.is_transparent);

    AppLauncher::with_window(window)
        .log_to_console()
        .launch(data)
        .expect("launch failed");
}

fn build_root_widget() -> impl Widget<ExampleData> {
    let decorations_button = Button::new("Decorations")
        .on_click(|ctx, data: &mut ExampleData, _env| {
            data.is_decorated = !data.is_decorated;
            ctx.window().show_decorations(data.is_decorated);
        });

    let titlebar_button = Button::new("Titlebar")
        .on_click(|ctx, data: &mut ExampleData, _env| {
            data.has_titlebar = !data.has_titlebar;
            ctx.window().show_titlebar(data.has_titlebar);
        });

    let resize_button = Button::new("Resize")
        .on_click(|ctx, data: &mut ExampleData, _env| {
            data.is_resizable = !data.is_resizable;
            ctx.window().resizable(data.is_resizable);
        });

    Flex::column()
        .with_child(Label::new("Click on any button to enable/disable a window feature"))
        .with_child(decorations_button)
        .with_child(titlebar_button)
        .with_child(resize_button)
}