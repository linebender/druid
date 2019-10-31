//! testing the ignore attribute

use druid::Data;

#[test]
fn simple_ignore() {
    #[derive(Clone, Data)]
    struct Point {
        x: f64,
        #[druid(ignore)]
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
        #[druid(ignore)]
        path: PathBuf,
    }
}
