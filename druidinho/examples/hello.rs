use druidinho::piet::Color;
use druidinho::widgets::{
    layout::{Align, Row, SizedBox},
    Background,
};

fn main() {
    let widget1 = SizedBox::empty().size((69., 69.));
    let widget1 = Background::new(widget1)
        .background(Color::PURPLE)
        .border(Color::YELLOW, 10.0);
    let widget1 = Align::new(widget1).centered();

    let widget2 = SizedBox::empty().size((69., 69.));
    let widget2 = Background::new(widget2)
        .background(Color::TEAL)
        .border(Color::BLUE, 5.0);
    let widget2 = Align::new(widget2).centered();

    let row = Row::new().with_child(widget1).with_child(widget2);
    druidinho::launch(row).unwrap()
}
