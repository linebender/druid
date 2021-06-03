use std::cell::Cell;
use std::rc::Rc;

use druidinho::piet::{Color, FontWeight};
use druidinho::widgets::{
    layout::{Column, Row, SizedBox},
    Button, Text, Updater,
};
use druidinho::WidgetExt;

#[derive(Clone, Debug, Default)]
struct AppState {
    count: Cell<u32>,
}

enum Actions {
    Increment,
    Decrement,
}

impl AppState {
    fn increment_count(&self) {
        let current = self.count.get();
        self.count.set(current + 1);
    }

    fn decrement_count(&self) {
        let current = self.count.get();
        self.count.set(current.saturating_sub(1));
    }
}

fn main() {
    let app_state = Rc::new(AppState::default());

    let widget1 = SizedBox::empty()
        .size((69., 69.))
        .background(Color::PURPLE)
        .border(Color::YELLOW, 10.0);

    let widget2 = SizedBox::empty()
        .size((69., 69.))
        .background(Color::TEAL)
        .border(Color::BLUE, 5.0);

    let widget3 = Text::new("hello");
    let mut widget4 = Text::new(" world");
    widget4.set_font_size(24.0);

    let update_app_state = app_state.clone();
    let update = Updater::new(widget4, move |chld| {
        if update_app_state.count.get() % 5 == 0 {
            chld.set_weight(FontWeight::EXTRA_BOLD);
        } else if update_app_state.count.get() % 2 == 0 {
            chld.set_weight(FontWeight::BOLD);
        } else {
            chld.set_weight(FontWeight::LIGHT);
        }
        chld.set_text(format!(" world #{}", update_app_state.count.get()));
    });

    let button_app_state = app_state.clone();
    let increment = Button::new("Increment").map_actions(|_| Actions::Increment);
    let decrement = Button::new("Decrement").map_actions(|_| Actions::Decrement);

    let row = Row::new()
        .with_child(widget1)
        .with_child(widget2)
        .with_child(widget3)
        .with_child(update);
    let col = Column::new()
        .with_child(row.suppress_actions())
        .with_child(increment)
        .with_child(SizedBox::empty().size((10., 10.0)).suppress_actions())
        .with_child(decrement)
        .center()
        .handle_actions(move |action, ctx| {
            match action {
                Actions::Increment => button_app_state.increment_count(),
                Actions::Decrement => button_app_state.decrement_count(),
            }
            ctx.request_update();
        });
    druidinho::launch(col).unwrap()
}
