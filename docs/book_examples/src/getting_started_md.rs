#![allow(clippy::let_unit_value)]

// ANCHOR: example_1_imports
use druid::widget::Label;
use druid::{AppLauncher, Widget, WindowDesc};
// ANCHOR_END: example_1_imports

// ANCHOR: example_2_imports
use druid::widget::{Container, Flex, Split};
use druid::Color;
// ANCHOR_END: example_2_imports

// ANCHOR: example_3_imports
use im::Vector;
// ANCHOR_END: example_3_imports

// ANCHOR: example_3b_imports
use druid::widget::List;
// ANCHOR_END: example_3b_imports

// ANCHOR: example_4_imports
use im::vector;
// ANCHOR_END: example_4_imports

// ANCHOR: example_5_imports
use druid::widget::Button;
// ANCHOR_END: example_5_imports

// ANCHOR: example_1
fn build_ui() -> impl Widget<()> {
    Label::new("Hello world")
}

fn main() {
    let main_window = WindowDesc::new(build_ui())
        .window_size((600.0, 400.0))
        .title("My first Druid App");
    let initial_data = ();

    AppLauncher::with_window(main_window)
        .launch(initial_data)
        .expect("Failed to launch application");
}
// ANCHOR_END: example_1

fn build_example_2() -> impl Widget<()> {
    // ANCHOR: example_2_builder
    Split::columns(
        Container::new(
            Flex::column()
                .with_flex_child(Label::new("first item"), 1.0)
                .with_flex_child(Label::new("second item"), 1.0)
                .with_flex_child(Label::new("third item"), 1.0)
                .with_flex_child(Label::new("fourth item"), 1.0),
        )
        .border(Color::grey(0.6), 2.0),
        Container::new(
            Flex::column()
                .with_flex_child(Label::new("Button placeholder"), 1.0)
                .with_flex_child(Label::new("Textbox placeholder"), 1.0),
        )
        .border(Color::grey(0.6), 2.0),
    )
    // ANCHOR_END: example_2_builder
}

type TodoList = Vector<String>;

fn build_example_3() -> impl Widget<TodoList> {
    // ANCHOR: example_3_builder
    Split::columns(
        Container::new(
            // Dynamic list of Widgets
            List::new(|| Label::dynamic(|data, _| format!("List item: {data}"))),
        )
        .border(Color::grey(0.6), 2.0),
        Container::new(
            Flex::column()
                .with_flex_child(Label::new("Button placeholder"), 1.0)
                .with_flex_child(Label::new("Textbox placeholder"), 1.0),
        )
        .border(Color::grey(0.6), 2.0),
    )
    // ANCHOR_END: example_3_builder
}

fn example_4_main() {
    fn build_ui() -> impl Widget<TodoList> {
        build_example_3()
    }

    // ANCHOR: example_4_main
    let main_window = WindowDesc::new(build_ui())
        .window_size((600.0, 400.0))
        .title("My first Druid App");
    let initial_data = vector![
        "first item".into(),
        "second item".into(),
        "third item".into(),
        "foo".into(),
        "bar".into(),
    ];

    AppLauncher::with_window(main_window)
        .launch(initial_data)
        .expect("Failed to launch application");
    // ANCHOR_END: example_4_main
}

fn build_example_5a() -> impl Widget<TodoList> {
    // ANCHOR: example_5a_button
    // Replace `Label::new("Button placeholder")` with
    Button::new("Add item")
    // ANCHOR_END: example_5a_button
}

fn build_example_5b() -> impl Widget<TodoList> {
    // ANCHOR: example_5b_button
    Button::new("Add item")
        .on_click(|_, data: &mut Vector<String>, _| data.push_back("New item".into()))
    // ANCHOR_END: example_5b_button
}
