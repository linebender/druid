// Copyright 2020 The Druid Authors.
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

//! Font attributes

use crate::{Data, FontFamily, FontStyle, FontWeight};

/// A collection of attributes that describe a font.
///
/// This is provided as a convenience; library consumers may wish to have
/// a single type that represents a specific font face at a specific size.
#[derive(Debug, Data, Clone, PartialEq)]
pub struct FontDescriptor {
    /// The font's [`FontFamily`](struct.FontFamily.html).
    pub family: FontFamily,
    /// The font's size.
    pub size: f64,
    /// The font's [`FontWeight`](struct.FontWeight.html).
    pub weight: FontWeight,
    /// The font's [`FontStyle`](struct.FontStyle.html).
    pub style: FontStyle,
}

impl FontDescriptor {
    /// Create a new descriptor with the provided [`FontFamily`].
    ///
    /// [`FontFamily`]: struct.FontFamily.html
    pub fn new(family: FontFamily) -> Self {
        FontDescriptor {
            family,
            ..Default::default()
        }
    }

    /// Buider-style method to set the descriptor's font size.
    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// Buider-style method to set the descriptor's [`FontWeight`].
    ///
    /// [`FontWeight`]: struct.FontWeight.html
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Buider-style method to set the descriptor's [`FontStyle`].
    ///
    /// [`FontStyle`]: enum.FontStyle.html
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }
}

impl Default for FontDescriptor {
    fn default() -> Self {
        FontDescriptor {
            family: Default::default(),
            weight: Default::default(),
            style: Default::default(),
            size: crate::piet::util::DEFAULT_FONT_SIZE,
        }
    }
}
