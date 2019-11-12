// Copyright 2019 The xi-editor Authors.
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

//! Convenience methods for widgets.

use crate::kurbo::Insets;
use crate::piet::{PaintBrush, UnitPoint};

use super::{Align, Container, EnvScope, Padding, SizedBox};
use crate::{Data, Env, Lens, LensWrap, Widget};

/// A trait that provides extra methods for combining `Widget`s.
pub trait WidgetExt<T: Data>: Widget<T> + Sized + 'static {
    /// Wrap this widget in a [`Padding`] widget with the given [`Insets`].
    ///
    /// [`Padding`]: struct.Padding.html
    /// [`Insets`]: https://docs.rs/kurbo/0.5.4/kurbo/struct.Insets.html
    fn padding(self, insets: impl Into<Insets>) -> Padding<T> {
        Padding::new(insets, self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to center it.
    ///
    /// [`Align`]: struct.Align.html
    fn center(self) -> Align<T> {
        Align::centered(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align left.
    ///
    /// [`Align`]: struct.Align.html
    fn align_left(self) -> Align<T> {
        Align::left(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align right.
    ///
    /// [`Align`]: struct.Align.html
    fn align_right(self) -> Align<T> {
        Align::right(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align vertically.
    ///
    /// [`Align`]: struct.Align.html
    fn align_vertical(self, align: UnitPoint) -> Align<T> {
        Align::vertical(align, self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align horizontally.
    ///
    /// [`Align`]: struct.Align.html
    fn align_horizontal(self, align: UnitPoint) -> Align<T> {
        Align::horizontal(align, self)
    }

    /// Wrap this widget in a [`SizedBox`] with an explicit width.
    ///
    /// [`SizedBox`]: struct.SizedBox.html
    fn fix_width(self, width: f64) -> SizedBox<T> {
        SizedBox::new(self).width(width)
    }

    /// Wrap this widget in a [`SizedBox`] with an explicit width.
    ///
    /// [`SizedBox`]: struct.SizedBox.html
    fn fix_height(self, height: f64) -> SizedBox<T> {
        SizedBox::new(self).height(height)
    }

    /// Wrap this widget in a [`Container`] using the provided [`PaintBrush`]
    /// as the background.
    ///
    /// The `PaintBrush` argument can be any color or gradient.
    ///
    /// [`Container`]: struct.Container.html
    /// [`PaintBrush`]: https://docs.rs/piet/0.0.7/piet/enum.PaintBrush.html
    fn background(self, brush: impl Into<PaintBrush>) -> Container<T> {
        Container::new(self).background(brush)
    }

    /// Wrap this widget in a [`Container`] with the given border.
    ///
    /// The `PaintBrush` argument can be any color or gradient.
    ///
    /// [`Container`]: struct.Container.html
    /// [`PaintBrush`]: https://docs.rs/piet/0.0.7/piet/enum.PaintBrush.html
    fn border(self, brush: impl Into<PaintBrush>, width: f64) -> Container<T> {
        Container::new(self).border(brush, width)
    }

    /// Wrap this widget in a [`EnvScope`] widget, modifying the parent
    /// [`Env`] with the provided closure.
    ///
    /// [`EnvScope`]: struct.Container.html
    /// [`Env`]: struct.Env.html
    fn env_scope(self, f: impl Fn(&mut Env) + 'static) -> EnvScope<T, Self> {
        EnvScope::new(f, self)
    }

    /// Wrap this widget in a [`LensWrap`] widget for the provided [`Lens`].
    ///
    /// [`LensWrap`]: struct.LensWrap.html
    /// [`Lens`]: trait.Lens.html
    fn lens<U: Data, L: Lens<T, U>>(self, lens: L) -> LensWrap<U, L, Self> {
        LensWrap::new(self, lens)
    }
}

impl<T: Data + 'static, W: Widget<T> + 'static> WidgetExt<T> for W {}

// these are 'soft overrides' of methods on WidgetExt; resolution
// will choose an impl on a type over an impl in a trait for methods with the same
// name.
impl<T: Data + 'static> Container<T> {
    pub fn with_background(self, brush: impl Into<PaintBrush>) -> Container<T> {
        self.background(brush)
    }

    pub fn bordered(self, brush: impl Into<PaintBrush>, width: f64) -> Container<T> {
        self.border(brush, width)
    }
}

impl<T: Data + 'static> SizedBox<T> {
    pub fn fixed_width(self, width: f64) -> SizedBox<T> {
        self.width(width)
    }

    pub fn fixed_height(self, height: f64) -> SizedBox<T> {
        self.height(height)
    }
}
