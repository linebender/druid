use druid::*;

#[derive(Lens)]
struct MyThing {
    field_1: i32,
    field_2: String,
}

fn main() {
    let _ = MyThing::field_1;
    let _ = MyThing::field_2;
}
