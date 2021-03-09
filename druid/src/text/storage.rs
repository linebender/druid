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
use crate::{Data, Env, EventCtx, MouseEvent, TextLayout};

/// A type that represents text that can be displayed.
pub trait TextStorage: PietTextStorage + Data {
    /// This allow you to store your data inside the `TextLayout`.
    type Data: Default + Clone;

    /// If this TextStorage object manages style spans, it should implement
    /// this method and update the provided builder with its spans, as required.
    #[allow(unused_variables)]
    fn add_attributes(&self, builder: PietTextLayoutBuilder, env: &Env) -> PietTextLayoutBuilder {
        builder
    }

    /// This method called after text layout is done.
    #[allow(unused_variables)]
    fn after_layout(&self, layout: &TextLayout<Self>, data: &mut Self::Data) {}

    /// This method is called on mouse clicks.
    #[allow(unused_variables)]
    fn mouse_click(&self, ctx: &mut EventCtx, event: &MouseEvent, data: &Self::Data, env: &Env) {}

    /// This method is called on mouse move.
    #[allow(unused_variables)]
    fn mouse_move(&self, ctx: &mut EventCtx, event: &MouseEvent, data: &Self::Data, env: &Env) {}
}

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in druid. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = Arc<str>;

impl TextStorage for ArcStr {
    type Data = ();
}

impl TextStorage for String {
    type Data = ();
}

impl TextStorage for Arc<String> {
    type Data = ();
}
