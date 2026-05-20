// Copyright 2022 The Druid Authors.
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

use std::any::Any;

use crate::{
    id::{Id, IdPath},
    view::View,
};

pub trait ViewTuple<T, A> {
    type State;

    type Elements;

    fn build(&self, id_path: &mut IdPath) -> (Self::State, Self::Elements);

    fn rebuild(
        &self,
        id_path: &mut IdPath,
        prev: &Self,
        state: &mut Self::State,
        els: &mut Self::Elements,
    );

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> A;
}

macro_rules! impl_view_tuple {
    ( $n: tt; $( $t:ident),* ; $( $s:tt ),* ) => {
        impl<T, A, $( $t: View<T, A> ),* > ViewTuple<T, A> for ( $( $t, )* ) {
            type State = ( $( $t::State, )* [Id; $n]);

            type Elements = ( $( $t::Element, )* );

            fn build(&self, id_path: &mut IdPath) -> (Self::State, Self::Elements) {
                let b = ( $( self.$s.build(id_path), )* );
                let state = ( $( b.$s.1, )* [ $( b.$s.0 ),* ]);
                let els = ( $( b.$s.2, )* );
                (state, els)
            }

            fn rebuild(
                &self,
                id_path: &mut IdPath,
                prev: &Self,
                state: &mut Self::State,
                els: &mut Self::Elements,
            ) {
                $(
                self.$s
                    .rebuild(id_path, &prev.$s, &mut state.$n[$s], &mut state.$s, &mut els.$s);
                )*
            }

            fn event(
                &self,
                id_path: &[Id],
                state: &mut Self::State,
                event: Box<dyn Any>,
                app_state: &mut T,
            ) -> A {
                let hd = id_path[0];
                let tl = &id_path[1..];
                $(
                if hd == state.$n[$s] {
                    self.$s.event(tl, &mut state.$s, event, app_state)
                } else )* {
                    panic!("inconsistent id_path")
                }
            }
        }
    }
}

impl_view_tuple!(1; V0; 0);
impl_view_tuple!(2; V0, V1; 0, 1);
impl_view_tuple!(3; V0, V1, V2; 0, 1, 2);
impl_view_tuple!(4; V0, V1, V2, V3; 0, 1, 2, 3);
impl_view_tuple!(5; V0, V1, V2, V3, V4; 0, 1, 2, 3, 4);
impl_view_tuple!(6; V0, V1, V2, V3, V4, V5; 0, 1, 2, 3, 4, 5);
impl_view_tuple!(7; V0, V1, V2, V3, V4, V5, V6; 0, 1, 2, 3, 4, 5, 6);
impl_view_tuple!(8;
    V0, V1, V2, V3, V4, V5, V6, V7;
    0, 1, 2, 3, 4, 5, 6, 7
);
impl_view_tuple!(9;
    V0, V1, V2, V3, V4, V5, V6, V7, V8;
    0, 1, 2, 3, 4, 5, 6, 7, 8
);
impl_view_tuple!(10;
    V0, V1, V2, V3, V4, V5, V6, V7, V8, V9;
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9
);
