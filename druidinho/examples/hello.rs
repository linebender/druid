use druidinho::piet::Color;
use druidinho::widgets::{layout::SizedBox, Background};

fn main() {
    let sized_box = SizedBox::empty().size((69., 69.));
    let widget = Background::new(sized_box)
        .background(Color::TEAL)
        .border(Color::BLUE, 10.0);
    druidinho::launch(widget).unwrap()
}
