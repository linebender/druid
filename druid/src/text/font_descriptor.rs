// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Font attributes

use crate::{Data, FontFamily, FontStyle, FontWeight};

/// A collection of attributes that describe a font.
///
/// This is provided as a convenience; library consumers may wish to have
/// a single type that represents a specific font face at a specific size.
#[derive(Debug, Data, Clone, PartialEq)]
pub struct FontDescriptor {
    /// The font's [`FontFamily`].
    pub family: FontFamily,
    /// The font's size.
    pub size: f64,
    /// The font's [`FontWeight`].
    pub weight: FontWeight,
    /// The font's [`FontStyle`].
    pub style: FontStyle,
}

impl FontDescriptor {
    /// Create a new descriptor with the provided [`FontFamily`].
    pub const fn new(family: FontFamily) -> Self {
        FontDescriptor {
            family,
            size: crate::piet::util::DEFAULT_FONT_SIZE,
            weight: FontWeight::REGULAR,
            style: FontStyle::Regular,
        }
    }

    /// Buider-style method to set the descriptor's font size.
    pub const fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// Buider-style method to set the descriptor's [`FontWeight`].
    pub const fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Buider-style method to set the descriptor's [`FontStyle`].
    pub const fn with_style(mut self, style: FontStyle) -> Self {
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
