use float_cmp::approx_eq;

use druid::Data;
use druid::Lens;

#[test]
fn derive_lens() {
    #[derive(Lens)]
    struct State {
        text: String,
        #[lens(name = "lens_number")]
        number: f64,
        #[lens(ignore)]
        ignored: f64,
    }

    let mut state = State {
        text: "1.0".into(),
        number: 1.0,
        ignored: 2.0,
    };

    let text_lens = State::text;
    let number_lens = State::lens_number; //named lens for number

    text_lens.with(&state, |data| assert_eq!(data, "1.0"));
    number_lens.with(&state, |data| approx_eq!(f64, *data, 1.0));

    text_lens.with_mut(&mut state, |data| *data = "2.0".into());
    number_lens.with_mut(&mut state, |data| *data = 2.0);

    assert_eq!(state.text, "2.0");
    approx_eq!(f64, state.number, 2.0);
    approx_eq!(f64, state.ignored, 2.0);
}

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
