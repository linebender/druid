use druid::Lens;
use druid::Prism;

#[test]
fn derive_lens() {
    #[derive(Lens)]
    struct State {
        text: String,
        alt: Alt,
    }

    #[allow(dead_code)]
    #[derive(Debug, PartialEq, Prism)]
    enum Alt {
        A(bool),
        B(u8),
    }

    let state_a = State {
        text: "1.0".into(),
        alt: Alt::B(2),
    };

    let text_lens = State::text;
    let alt_lens = State::alt;
    let alt_a_prism = Alt::a;
    let alt_b_prism = Alt::b;

    let f_alt_b = |data: &u8| data + 3;
    let f_alt = |data: &Alt| alt_b_prism.with(data, f_alt_b);
    let result: Option<u8> = alt_lens.with(&state_a, f_alt);
    assert_eq!(Some(2 + 3), result);

    text_lens.with(&state_a, |data: &String| assert_eq!(data, "1.0"));
    alt_lens.with(&state_a, |data: &Alt| assert_eq!(data, &Alt::B(2)));
    alt_lens.with(&state_a, |data: &Alt| assert_ne!(data, &Alt::A(true)));
    alt_lens.with(&state_a, |data: &Alt| assert_ne!(data, &Alt::A(false)));

    let txt: String = text_lens.with(&state_a, |data: &String| data.clone());
    assert_eq!("1.0", &txt);

    alt_lens.with(&state_a, |data: &Alt| {
        alt_a_prism.with(data, |data| assert_eq!(data, &true))
    });

    let b: Option<bool> =
        alt_lens.with(&state_a, |data: &Alt| alt_a_prism.with(data, |data| *data));
    assert_eq!(None, b);

    alt_lens.with(&state_a, |data: &Alt| {
        alt_a_prism.with(data, |data| assert_ne!(data, &false))
    });

    alt_lens.with(&state_a, |data: &Alt| {
        alt_a_prism.with(data, |_data| panic!())
    });

    let u: Option<bool> =
        alt_lens.with(&state_a, |data: &Alt| alt_a_prism.with(data, |data| *data));
    assert_eq!(None, u);
}
