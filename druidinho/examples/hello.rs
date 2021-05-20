use druidinho::piet::Color;
use druidinho::widgets::{
    layout::{Align, SizedBox},
    Background,
};

fn main() {
    let widget = SizedBox::empty().size((69., 69.));
    let widget = Background::new(widget)
        .background(Color::TEAL)
        .border(Color::BLUE, 5.0);
    let widget = Align::new(widget).centered();
    druidinho::launch(widget).unwrap()
}
