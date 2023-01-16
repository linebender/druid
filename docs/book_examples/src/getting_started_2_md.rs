#![allow(unused)]

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

// ---

// ANCHOR: example_6_imports
use druid::{Data, Lens};
// ANCHOR_END: example_6_imports

// ANCHOR: example_6_derive
#[derive(Clone, Data, Lens)]
// ANCHOR_END: example_6_derive

// ANCHOR: example_6_struct
struct TodoList {
    items: Vector<String>,
    next_item: String,
}
// ANCHOR_END: example_6_struct

// ANCHOR: example_7_imports
use druid::widget::LensWrap;
// ANCHOR_END: example_7_imports

fn build_example_7() -> impl Widget<TodoList> {
    // ANCHOR: example_7
    // Replace previous List with:
    LensWrap::new(
        List::new(|| Label::dynamic(|data, _| format!("List item: {data}"))),
        TodoList::items,
    )
    // ANCHOR_END: example_7
}

fn build_example_7b() -> impl Widget<TodoList> {
    // ANCHOR: example_7b
    // Replace previous Button with:
    Button::new("Add item").on_click(|_, data: &mut TodoList, _| {
        data.items.push_back(data.next_item.clone());
        data.next_item = String::new();
    })
    // ANCHOR_END: example_7b
}

// ANCHOR: example_8_imports
use druid::widget::TextBox;
// ANCHOR_END: example_8_imports

fn build_example_8() -> impl Widget<TodoList> {
    // ANCHOR: example_8
    // Replace `Label::new("Textbox placeholder")` with
    LensWrap::new(TextBox::new(), TodoList::next_item)
    // ANCHOR_END: example_8
}

// ANCHOR: complete_code
fn build_ui() -> impl Widget<TodoList> {
    Split::columns(
        Container::new(
            // Dynamic list of Widgets
            LensWrap::new(
                List::new(|| Label::dynamic(|data, _| format!("List item: {data}"))),
                TodoList::items,
            ),
        )
        .border(Color::grey(0.6), 2.0),
        Container::new(
            Flex::column()
                .with_flex_child(
                    Button::new("Add item").on_click(|_, data: &mut TodoList, _| {
                        data.items.push_back(data.next_item.clone());
                        data.next_item = String::new();
                    }),
                    1.0,
                )
                .with_flex_child(LensWrap::new(TextBox::new(), TodoList::next_item), 1.0),
        )
        .border(Color::grey(0.6), 2.0),
    )
}

fn main() {
    let main_window = WindowDesc::new(build_ui())
        .window_size((600.0, 400.0))
        .title("My first Druid App");
    let initial_data = TodoList {
        items: vector![
            "first item".into(),
            "second item".into(),
            "third item".into(),
            "foo".into(),
            "bar".into(),
        ],
        next_item: String::new(),
    };

    AppLauncher::with_window(main_window)
        .launch(initial_data)
        .expect("Failed to launch application");
}
// ANCHOR_END: complete_code
