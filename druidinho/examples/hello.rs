use druidinho::piet::{Color, FontWeight};
use druidinho::widgets::{
    layout::{Column, Row, SizedBox},
    Button, Text, Updater,
};
use druidinho::{App, LaunchCtx, Widget, WidgetExt};

#[derive(Clone, Debug, Default)]
struct AppState {
    count: u32,
}

enum Actions {
    Increment,
    Decrement,
}

impl App for AppState {
    type Action = Actions;

    fn update(&mut self, actions: &[Self::Action], update: &mut bool) {
        for action in actions {
            match action {
                Actions::Increment => {
                    let _ = self.count.saturating_add(1);
                }
                Actions::Decrement => {
                    let _ = self.count.saturating_sub(1);
                }
            }
        }
        *update = true;
    }

    fn launch(&mut self, _ctx: &mut LaunchCtx) -> Box<dyn Widget<Action = Actions>> {
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

        //let update_app_state = app_state.clone();
        //let update = Updater::new(widget4, move |chld| {
        //if update_app_state.count.get() % 5 == 0 {
        //chld.set_weight(FontWeight::EXTRA_BOLD);
        //} else if update_app_state.count.get() % 2 == 0 {
        //chld.set_weight(FontWeight::BOLD);
        //} else {
        //chld.set_weight(FontWeight::LIGHT);
        //}
        //chld.set_text(format!(" world #{}", update_app_state.count.get()));
        //});

        //let button_app_state = app_state.clone();
        let increment = Button::new("Increment").map_actions(|_| Actions::Increment);
        let decrement = Button::new("Decrement").map_actions(|_| Actions::Decrement);

        let row = Row::new()
            .with_child(widget1)
            .with_child(widget2)
            .with_child(widget3);
        //.with_child(update);
        let col = Column::new()
            .with_child(row.suppress_actions())
            .with_child(increment)
            .with_child(SizedBox::empty().size((10., 10.0)).suppress_actions())
            .with_child(decrement)
            .center();
        Box::new(col)
    }
}

fn main() {
    let app_state = AppState::default();

    druidinho::launch(app_state).unwrap();

    // okay, so what do we *want* this to look like?
}
