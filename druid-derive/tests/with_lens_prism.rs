use float_cmp::approx_eq;

use druid::lens;
use druid::Data;
use druid::Lens;
use druid::Prism;

#[test]
fn derive_lens() {
    #[derive(Lens)]
    struct State {
        text: String,
        alt: Alt,
    }

    #[derive(Debug, PartialEq, Prism)]
    enum Alt {
        A(bool),
        B(u8),
    }

    let mut state_a = State {
        text: "1.0".into(),
        alt: Alt::A(true),
    };

    let text_lens = State::text;
    let alt_lens = State::alt; //named lens for number
    let alt_a_prism = Alt::a;
    let alt_b_prism = Alt::b;

    // let x = Alt::a;
    // let alt_a_lens = {
    //     use druid::LensExt;
    //     // cannot, as Alt::a isn't a lens
    //     lens!(State, alt).then(Alt::a)
    // };

    text_lens.with(&state_a, |data: &String| assert_eq!(data, "1.0"));
    alt_lens.with(&state_a, |data: &Alt| assert_eq!(data, &Alt::A(true)));

    let txt: String = text_lens.with(&state_a, |data: &String| data.clone());
    assert_eq!("1.0", &txt);

    alt_lens.with(&state_a, |data: &Alt| {
        alt_a_prism.with(data, |data| assert_eq!(data, &true))
    });

    let b: Option<bool> =
        alt_lens.with(&state_a, |data: &Alt| alt_a_prism.with(data, |data| *data));
    assert_eq!(Some(true), b);

    alt_lens.with(&state_a, |data: &Alt| {
        alt_a_prism.with(data, |data| assert_ne!(data, &false))
    });

    alt_lens.with(&state_a, |data: &Alt| {
        alt_b_prism.with(data, |_data| panic!())
    });

    let u: Option<u8> = alt_lens.with(&state_a, |data: &Alt| alt_b_prism.with(data, |data| *data));
    assert_eq!(None, u);
}
