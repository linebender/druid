use druid::Lens;

#[test]
fn derive_lens() {
    #[derive(Clone, Lens, Debug)]
    struct Foo {
        text: String,
        #[druid(lens_name = "lens_number")]
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
