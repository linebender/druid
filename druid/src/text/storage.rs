// Copyright 2020 The xi-editor Authors.
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

//! Storing text.

use std::sync::Arc;

use crate::piet::{PietTextLayoutBuilder, TextStorage as PietTextStorage};
use crate::{Data, Env, Event, EventCtx, TextLayout};

/// A type that represents text that can be displayed.
pub trait TextStorage: PietTextStorage + Data {
    /// If this TextStorage object manages style spans, it should implement
    /// this method and update the provided builder with its spans, as required.
    #[allow(unused_variables)]
    fn add_attributes(&self, builder: PietTextLayoutBuilder, env: &Env) -> PietTextLayoutBuilder {
        builder
    }

    #[allow(unused_variables)]
    fn event(&self, ctx: &mut EventCtx, event: &Event, layout: &mut TextLayout<Self>, env: &Env) {}
}

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in druid. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = Arc<str>;

impl TextStorage for ArcStr {}

impl TextStorage for String {}

impl TextStorage for Arc<String> {}
