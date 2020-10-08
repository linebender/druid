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

use std::ops::RangeBounds;
use std::sync::Arc;

use super::{Attribute, AttributeSpans};
use crate::piet::{util, PietTextLayoutBuilder, TextLayoutBuilder, TextStorage as PietTextStorage};
use crate::{Data, Env};

/// A type that represents text that can be displayed.
pub trait TextStorage: PietTextStorage + Data {
    /// If this TextStorage object manages style spans, it should implement
    /// this method and update the provided builder with its spans, as required.
    #[allow(unused_variables)]
    fn add_attributes(&self, builder: PietTextLayoutBuilder, env: &Env) -> PietTextLayoutBuilder {
        builder
    }
}

/// A reference counted string slice.
///
/// This is a data-friendly way to represent strings in druid. Unlike `String`
/// it cannot be mutated, but unlike `String` it can be cheaply cloned.
pub type ArcStr = Arc<str>;

/// Text with optional style spans.
#[derive(Debug, Clone, Data)]
pub struct RichText {
    buffer: ArcStr,
    attrs: Arc<AttributeSpans>,
}

impl RichText {
    /// Create a new `RichText` object with the provided text.
    pub fn new(buffer: ArcStr) -> Self {
        RichText {
            buffer,
            attrs: Arc::new(AttributeSpans::default()),
        }
    }

    /// Builder-style method for adding an [`Attribute`] to a range of text.
    ///
    /// [`Attribute`]: enum.Attribute.html
    pub fn with_attribute(mut self, range: impl RangeBounds<usize>, attr: Attribute) -> Self {
        self.add_attribute(range, attr);
        self
    }

    /// The length of the buffer, in utf8 code units.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns `true` if the underlying buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Add an [`Attribute`] to the provided range of text.
    ///
    /// [`Attribute`]: enum.Attribute.html
    pub fn add_attribute(&mut self, range: impl RangeBounds<usize>, attr: Attribute) {
        let range = util::resolve_range(range, self.buffer.len());
        Arc::make_mut(&mut self.attrs).add(range, attr);
    }
}

impl TextStorage for ArcStr {}

impl PietTextStorage for RichText {
    fn as_str(&self) -> &str {
        self.buffer.as_str()
    }
}

impl TextStorage for String {}

impl TextStorage for Arc<String> {}

impl TextStorage for RichText {
    fn add_attributes(
        &self,
        mut builder: PietTextLayoutBuilder,
        env: &Env,
    ) -> PietTextLayoutBuilder {
        for (range, attr) in self.attrs.to_piet_attrs(env) {
            builder = builder.range_attribute(range, attr);
        }
        builder
    }
}
