use float_cmp::approx_eq;

use druid::Data;
use druid::Prism;

#[test]
fn derive_prism() {
    use druid::prism;

    #[derive(Debug, Prism, PartialEq)]
    pub enum State {
        Text(String),
        #[prism(name = "prism_number")]
        Number(f64),
    }

    let mut state_text = State::Text("1.0".into());
    let mut state_number = State::Number(1.0);

    let text_prism = State::text;
    let number_prism = State::prism_number;

    text_prism.with(&state_text, |data| assert_eq!(data, "1.0"));
    number_prism.with(&state_number, |data| assert!(approx_eq!(f64, *data, 1.0)));

    // tests the prism! macro
    prism!(State, Text).with(&state_text, |data| assert_eq!(data, "1.0"));
    prism!(State, Number).with(&state_number, |data| assert!(approx_eq!(f64, *data, 1.0)));

    // mappings for the wrong variant are ignored
    assert_eq!(None, text_prism.with(&state_number, |_data| unreachable!()));
    assert_eq!(None, number_prism.with(&state_text, |_data| unreachable!()));

    assert_eq!(
        Some(()),
        text_prism.with_mut(&mut state_text, |data| *data = "2.0".into())
    );
    assert_eq!(
        Some(()),
        number_prism.with_mut(&mut state_number, |data| *data = 2.0)
    );

    assert_ne!(state_text, State::Text("1.0".into()));
    assert_eq!(state_text, State::Text("2.0".into()));

    let num: f64 = if let State::Number(f) = state_number {
        f
    } else {
        unreachable!()
    };

    assert!(!approx_eq!(f64, num, 1.0));
    assert!(approx_eq!(f64, num, 2.0));

    {
        use druid::optics::prism::PrismReplacer;

        impl Default for State {
            fn default() -> Self {
                Self::Number(0.)
            }
        }

        // test upgrade
        assert_ne!(state_text, text_prism.upgrade("1.0".into()));
        assert_eq!(state_text, text_prism.upgrade("2.0".into()));

        // test replace
        // different from with_mut, wrong mapping re-builds the enum
        text_prism.replace(&mut state_text, "3.0".into());
        text_prism.replace(&mut state_number, "3.0".into());

        assert_eq!(state_text, State::Text("3.0".into()));
        assert_eq!(state_text, state_number);
    }
}

#[test]
fn named_derive_prism() {
    #[derive(Debug, Prism, PartialEq)]
    pub enum State {
        Text {
            s: String,
        },
        #[prism(name = "prism_number")]
        Number {
            n: f64,
        },
    }

    let mut state_text = State::Text { s: "1.0".into() };
    let mut state_number = State::Number { n: 1.0 };

    // let text_prism = druid::prism!(State, text);
    let text_prism = State::text;
    let number_prism = State::prism_number;

    assert_eq!(
        Some(()),
        text_prism.with(&state_text, |data| assert_eq!(data, "1.0"))
    );
    assert_eq!(
        Some(()),
        number_prism.with(&state_number, |data| assert!(approx_eq!(f64, *data, 1.0)))
    );

    // mappings for the wrong variant are ignored
    assert_eq!(None, text_prism.with(&state_number, |_data| unreachable!()));
    assert_eq!(None, number_prism.with(&state_text, |_data| unreachable!()));

    assert_eq!(
        Some(()),
        text_prism.with_mut(&mut state_text, |data| *data = "2.0".into())
    );
    assert_eq!(
        Some(()),
        number_prism.with_mut(&mut state_number, |data| *data = 2.0)
    );

    assert_ne!(state_text, State::Text { s: "1.0".into() });
    assert_eq!(state_text, State::Text { s: "2.0".into() });

    let num: f64 = if let State::Number { n: f } = state_number {
        f
    } else {
        unreachable!()
    };

    assert!(!approx_eq!(f64, num, 1.0));
    assert!(approx_eq!(f64, num, 2.0));
}

#[test]
fn mix_with_data_prism() {
    #[derive(Clone, Prism, Data)]
    enum State {
        // ignoring a variant makes it always
        // the same as any other variant
        #[data(ignore)]
        Text(String),
        #[prism(name = "prism_number")]
        Number(#[data(same_fn = "same_sign")] f64),
    }

    // test prism
    let mut state_text = State::Text("1.0".into());
    let mut state_number = State::Number(1.0);

    let text_prism = State::text;
    let number_prism = State::prism_number;

    assert_eq!(
        Some(()),
        text_prism.with(&state_text, |data| assert_eq!(data, "1.0"))
    );
    assert_eq!(
        Some(()),
        number_prism.with(&state_number, |data| assert!(approx_eq!(f64, *data, 1.0)))
    );

    // mappings for the wrong variant are ignored
    assert_eq!(None, text_prism.with(&state_number, |_data| unreachable!()));
    assert_eq!(None, number_prism.with(&state_text, |_data| unreachable!()));

    assert_eq!(
        Some(()),
        text_prism.with_mut(&mut state_text, |data| *data = "2.0".into())
    );
    assert_eq!(
        Some(()),
        number_prism.with_mut(&mut state_number, |data| *data = 2.0)
    );

    let num: f64 = if let State::Number(f) = state_number {
        f
    } else {
        unreachable!()
    };
    assert!(!approx_eq!(f64, num, 1.0));
    assert!(approx_eq!(f64, num, 2.0));

    // test data
    let two_text = State::Text("666".into());
    let two_number = State::Number(200.0);

    assert!(state_text.same(&two_text));
    assert!(state_number.same(&two_number));

    // ignored variants are always the same as any other variant
    assert!(state_text.same(&two_number));
    assert!(state_number.same(&two_text));
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn same_sign(one: &f64, two: &f64) -> bool {
    one.signum() == two.signum()
}
