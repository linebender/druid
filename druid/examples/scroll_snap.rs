use std::sync::Arc;

use druid::widget::{Button, Flex, Label, List, RadioGroup, Scroll};
use druid::{
    theme, AppLauncher, Color, Data, Env, FontDescriptor, FontWeight, Lens, Widget, WidgetExt,
    WindowDesc,
};

const WINDOW_SIZE: f64 = 200.0;
const WIDGET_SPACING: f64 = 20.0;
const COLOR_CLEAR: Color = Color::rgb8(0xff, 0x30, 0x30);

#[derive(Clone, Data, Lens)]
struct ScrollSnap {
    a_long_list_of_numbers: Arc<Vec<ScrollSnapMember>>,
    next_number: u32,
    snap: bool,
}

#[derive(Clone, Data)]
struct ScrollSnapMember {
    a_single_number: u32,
}

fn main() {
    let window = WindowDesc::new(build_window())
        .title("Window Snap")
        .window_size((WINDOW_SIZE, WINDOW_SIZE));

    let initial_state = ScrollSnap {
        a_long_list_of_numbers: Arc::new(Vec::new()),
        next_number: 0,
        snap: true,
    };

    AppLauncher::with_window(window)
        .configure_env(|env, _| env.set(theme::UI_FONT, FontDescriptor::default()))
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_window() -> impl Widget<ScrollSnap> {
    Flex::column()
        .with_child(build_button_add())
        .with_flex_child(build_list(), 1.0)
        .with_child(build_buttons_snap_subtract())
}

fn build_button_add() -> impl Widget<ScrollSnap> {
    Button::new("Click me!").on_click(|_, data: &mut ScrollSnap, _| {
        Arc::make_mut(&mut data.a_long_list_of_numbers).push(ScrollSnapMember {
            a_single_number: data.next_number,
        });
        data.next_number += 1;
    })
}

fn build_list() -> impl Widget<ScrollSnap> {
    Flex::column().with_flex_child(
        Scroll::new(List::new(build_individual_item).lens(ScrollSnap::a_long_list_of_numbers))
            .vertical()
            .with_snap_vertical(|data: &ScrollSnap, _| data.snap),
        1.0,
    )
}

fn build_individual_item() -> impl Widget<ScrollSnapMember> {
    Label::new(|data: &ScrollSnapMember, _env: &Env| data.a_single_number.to_string())
        .fix_width(40.0)
}

fn build_buttons_snap_subtract() -> impl Widget<ScrollSnap> {
    let pause = RadioGroup::new(vec![("||", false)]).lens(ScrollSnap::snap);

    let snap = RadioGroup::new(vec![("▼", true)]).lens(ScrollSnap::snap);

    let clear_x = Label::new("X")
        .with_font(FontDescriptor::default().with_weight(FontWeight::BOLD))
        .with_text_color(COLOR_CLEAR);

    let clear_button = Button::from_label(clear_x).on_click(move |_, data: &mut ScrollSnap, _| {
        data.a_long_list_of_numbers = Arc::new(Vec::new())
    });

    Flex::row()
        .with_child(pause)
        .with_spacer(WIDGET_SPACING)
        .with_child(snap)
        .with_spacer(WIDGET_SPACING)
        .with_child(clear_button)
}
