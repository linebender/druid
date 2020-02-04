use druid::lens::{Id, Tuple};
use druid::widget::{Button, Flex, Label, TextBox, ViewSwitcher, WidgetExt};
use druid::{AppLauncher, Data, Env, Lens, LensExt, LocalizedString, Widget, WindowDesc};
use std::sync::Arc;

#[derive(Clone, Data, Lens)]
struct AppState {
    current_view: u32,
    current_text: String,
    numbers: Arc<Vec<u32>>,
}

fn main() {
    let main_window = WindowDesc::new(make_ui).title(LocalizedString::new("View Switcher"));
    let data = AppState {
        current_view: 0,
        current_text: "Edit me!".to_string(),
        numbers: Arc::new((0..5).collect()),
    };
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    let mut switcher_column = Flex::column();
    switcher_column.add_child(
        Label::new(|data: &u32, _env: &Env| format!("Current view: {}", data))
            .lens(AppState::current_view),
        0.0,
    );
    for i in 0..5 {
        switcher_column.add_child(
            Button::<u32>::new(format!("View {}", i), move |_event, data, _env| {
                *data = i;
            })
            .lens(AppState::current_view),
            0.0,
        );
    }

    let view_switcher_lens = Tuple::new(AppState::current_view, Id);

    let view_switcher = ViewSwitcher::new(|data: &(u32, AppState), _env| {
        let current_text_lens = Id.map(
            |data: &(u32, AppState)| String::from(&data.1.current_text),
            |data, text| data.1.current_text = text,
        );

        match data.0 {
            0 => Box::new(Label::new("Simple Label").center()),
            1 => Box::new(Button::new(
                "Another Simple Button",
                |_event, _data, _env| {
                    println!("Simple button clicked!");
                },
            )),
            2 => Box::new(Button::new("Simple Button", |_event, _data, _env| {
                println!("Simple button clicked!");
            })),
            3 => Box::new(
                Flex::column()
                    .with_child(Label::new("Here is a label").center(), 1.0)
                    .with_child(
                        Button::new("Button", |_event, _data, _env| {
                            println!("Complex button clicked!");
                        }),
                        1.0,
                    )
                    .with_child(TextBox::new().lens(current_text_lens.clone()), 1.0)
                    .with_child(
                        Label::new(|data: &String, _env: &Env| format!("Value entered: {}", data))
                            .lens(current_text_lens.clone()),
                        1.0,
                    ),
            ),
            _ => Box::new(Label::new("Unknown").center()),
        }
    })
    .lens(view_switcher_lens);

    Flex::row()
        .with_child(switcher_column, 0.0)
        .with_child(view_switcher, 1.0)
}
