// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use druid::Data;

#[test]
fn same_fn() {
    #[derive(Clone, Data)]
    struct Nanana {
        bits: f64,
        #[data(eq)]
        peq: f64,
    }

    let one = Nanana {
        bits: 1.0,
        peq: std::f64::NAN,
    };
    let two = Nanana {
        bits: 1.0,
        peq: std::f64::NAN,
    };

    //according to partialeq, two NaNs are never equal
    assert!(!one.same(&two));

    let one = Nanana {
        bits: std::f64::NAN,
        peq: 1.0,
    };
    let two = Nanana {
        bits: std::f64::NAN,
        peq: 1.0,
    };

    // the default 'same' impl uses bitwise equality, so two bitwise-equal NaNs are equal
    assert!(one.same(&two));
}

#[test]
fn enums() {
    #[derive(Debug, Clone, Data)]
    enum Hi {
        One {
            bits: f64,
        },
        Two {
            #[data(same_fn = "same_sign")]
            bits: f64,
        },
        Tri(#[data(same_fn = "same_sign")] f64),
    }

    let oneone = Hi::One {
        bits: std::f64::NAN,
    };
    let onetwo = Hi::One {
        bits: std::f64::NAN,
    };
    assert!(oneone.same(&onetwo));

    let twoone = Hi::Two { bits: -1.1 };
    let twotwo = Hi::Two {
        bits: std::f64::NEG_INFINITY,
    };
    assert!(twoone.same(&twotwo));

    let trione = Hi::Tri(1001.);
    let tritwo = Hi::Tri(-1.);
    assert!(!trione.same(&tritwo));
}
#[allow(clippy::trivially_copy_pass_by_ref)]
fn same_sign(one: &f64, two: &f64) -> bool {
    one.signum() == two.signum()
}
