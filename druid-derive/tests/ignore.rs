//! testing the ignore attribute

use druid::Data;

#[test]
fn simple_ignore() {
    #[derive(Clone, Data)]
    struct Point {
        x: f64,
        #[data(ignore)]
        y: f64,
    }
    let p1 = Point { x: 0.0, y: 1.0 };
    let p2 = Point { x: 0.0, y: 9.0 };
    assert!(p1.same(&p2));
}

#[test]
fn ignore_item_without_data_impl() {
    use std::path::PathBuf;

    #[derive(Clone, Data)]
    struct CoolStruct {
        len: usize,
        #[data(ignore)]
        path: PathBuf,
    }
}

#[test]
fn tuple_struct() {
    #[derive(Clone, Data)]
    struct Tup(usize, #[data(ignore)] usize);

    let one = Tup(1, 1);
    let two = Tup(1, 5);
    assert!(one.same(&two));
}

#[test]
fn enums() {
    #[derive(Clone, Data)]
    enum Hmm {
        Named {
            one: usize,
            #[data(ignore)]
            two: usize,
        },
        Tuple(#[data(ignore)] usize, usize),
    }

    let name_one = Hmm::Named { one: 5, two: 4 };
    let name_two = Hmm::Named { one: 5, two: 42 };
    let tuple_one = Hmm::Tuple(2, 4);
    let tuple_two = Hmm::Tuple(9, 4);

    assert!(!name_one.same(&tuple_one));
    assert!(name_one.same(&name_two));
    assert!(tuple_one.same(&tuple_two));
}
