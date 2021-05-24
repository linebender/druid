use druidinho::piet::{Color, FontWeight};
use druidinho::widgets::{
    layout::{Align, Column, Row, SizedBox},
    Background, Button, Text,
};

fn main() {
    let widget1 = SizedBox::empty().size((69., 69.));
    let widget1 = Background::new(widget1)
        .background(Color::PURPLE)
        .border(Color::YELLOW, 10.0);

    let widget2 = SizedBox::empty().size((69., 69.));
    let widget2 = Background::new(widget2)
        .background(Color::TEAL)
        .border(Color::BLUE, 5.0);

    let widget3 = Text::new("hello");
    let widget4 = Text::new(" world").font_size(24.0).weight(FontWeight::BOLD);
    let button = Button::new("Click").on_click(|| println!("clicked"));

    let row = Row::new()
        .with_child(widget1)
        .with_child(widget2)
        .with_child(widget3)
        .with_child(widget4);
    let col = Column::new().with_child(row).with_child(button);
    druidinho::launch(Align::new(col).centered()).unwrap()
}
