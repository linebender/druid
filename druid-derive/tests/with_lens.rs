use druid::Data;
use druid::Lens;

#[test]
fn derive_lens() {
    #[derive(Lens)]
    struct Foo {
        text: String,
        #[lens(name = "lens_number")]
        number: f64,
    }

    let mut foo = Foo {
        text: "1.0".into(),
        number: 1.0,
    };

    let text_lens = Foo::text;
    let number_lens = Foo::lens_number; //named lens for number

    text_lens.with(&foo, |data| assert_eq!(data, "1.0"));
    number_lens.with(&foo, |data| assert_eq!(*data, 1.0));

    text_lens.with_mut(&mut foo, |data| *data = "2.0".into());
    number_lens.with_mut(&mut foo, |data| *data = 2.0);

    assert_eq!(foo.text, "2.0");
    assert_eq!(foo.number, 2.0);
}

#[test]
fn mix_with_data_lens() {
    #[derive(Clone, Lens, Data)]
    struct Foo {
        #[data(ignore)]
        text: String,
        #[data(same_fn = "same_sign")]
        #[lens(name = "lens_number")]
        number: f64,
    }

    //test lens
    let mut foo = Foo {
        text: "1.0".into(),
        number: 1.0,
    };
    let text_lens = Foo::text;
    let number_lens = Foo::lens_number; //named lens for number

    text_lens.with(&foo, |data| assert_eq!(data, "1.0"));
    number_lens.with(&foo, |data| assert_eq!(*data, 1.0));

    text_lens.with_mut(&mut foo, |data| *data = "2.0".into());
    number_lens.with_mut(&mut foo, |data| *data = 2.0);

    assert_eq!(foo.text, "2.0");
    assert_eq!(foo.number, 2.0);

    //test data
    let two = Foo {
        text: "666".into(),
        number: 200.0,
    };
    assert!(foo.same(&two))
}
fn same_sign(one: &f64, two: &f64) -> bool {
    one.signum() == two.signum()
}
