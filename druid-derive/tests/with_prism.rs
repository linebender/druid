use float_cmp::approx_eq;

use druid::Data;
use druid::Lens;
use druid::Prism;

#[test]
fn derive_prism() {
    #[derive(Debug, Prism, PartialEq)]
    pub enum State {
        Text(String),
        Number(f64),
    }

    let mut state_text = State::Text("1.0".into());
    let mut state_number = State::Number(1.0);

    let text_prism = State::text;
    let number_prism = State::number;

    text_prism.with(&state_text, |data| assert_eq!(data, "1.0"));
    number_prism.with(&state_number, |data| assert!(approx_eq!(f64, *data, 1.0)));

    // mappings for the wrong variant are ignored
    text_prism.with(&state_number, |_data| panic!());
    number_prism.with(&state_text, |_data| panic!());

    text_prism.with_mut(&mut state_text, |data| *data = "2.0".into());
    number_prism.with_mut(&mut state_number, |data| *data = 2.0);

    assert_ne!(state_text, State::Text("1.0".into()));
    assert_eq!(state_text, State::Text("2.0".into()));

    let num: f64 = if let State::Number(f) = state_number {
        f
    } else {
        panic!()
    };

    assert!(!approx_eq!(f64, num, 1.0));
    assert!(approx_eq!(f64, num, 2.0));
}

#[test]
fn named_derive_prism() {
    #[derive(Debug, Prism, PartialEq)]
    pub enum State {
        Text { s: String },
        Number { n: f64 },
    }

    let mut state_text = State::Text { s: "1.0".into() };
    let mut state_number = State::Number { n: 1.0 };

    let text_prism = State::text;
    let number_prism = State::number;

    text_prism.with(&state_text, |data| assert_eq!(data, "1.0"));
    number_prism.with(&state_number, |data| assert!(approx_eq!(f64, *data, 1.0)));

    // mappings for the wrong variant are ignored
    text_prism.with(&state_number, |_data| panic!());
    number_prism.with(&state_text, |_data| panic!());

    text_prism.with_mut(&mut state_text, |data| *data = "2.0".into());
    number_prism.with_mut(&mut state_number, |data| *data = 2.0);

    assert_ne!(state_text, State::Text { s: "1.0".into() });
    assert_eq!(state_text, State::Text { s: "2.0".into() });

    let num: f64 = if let State::Number { n: f } = state_number {
        f
    } else {
        panic!()
    };

    assert!(!approx_eq!(f64, num, 1.0));
    assert!(approx_eq!(f64, num, 2.0));
}

/*
#[test]
fn mix_with_data_lens() {
    #[derive(Clone, Lens, Data)]
    struct State {
        #[data(ignore)]
        text: String,
        #[data(same_fn = "same_sign")]
        #[lens(name = "lens_number")]
        number: f64,
    }

    //test lens
    let mut state = State {
        text: "1.0".into(),
        number: 1.0,
    };
    let text_lens = State::text;
    let number_lens = State::lens_number; //named lens for number

    text_lens.with(&state, |data| assert_eq!(data, "1.0"));
    number_lens.with(&state, |data| approx_eq!(f64, *data, 1.0));

    text_lens.with_mut(&mut state, |data| *data = "2.0".into());
    number_lens.with_mut(&mut state, |data| *data = 2.0);

    assert_eq!(state.text, "2.0");
    approx_eq!(f64, state.number, 2.0);

    //test data
    let two = State {
        text: "666".into(),
        number: 200.0,
    };
    assert!(state.same(&two))
}
#[allow(clippy::trivially_copy_pass_by_ref)]
fn same_sign(one: &f64, two: &f64) -> bool {
    one.signum() == two.signum()
}

*/
