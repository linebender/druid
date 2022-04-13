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

pub mod adapt;
pub mod any_view;
pub mod button;
pub mod column;
pub mod memoize;
pub mod use_state;

use std::any::Any;

use crate::id::{Id, IdPath};

pub trait View<T, A> {
    type State;

    type Element;

    fn build(&self, id_path: &mut IdPath) -> (Id, Self::State, Self::Element);

    fn rebuild(
        &self,
        id_path: &mut IdPath,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    );

    fn event(
        &self,
        id_path: &[Id],
        state: &Self::State,
        event: Box<dyn Any>,
        app_state: &mut T,
    ) -> A;
}
