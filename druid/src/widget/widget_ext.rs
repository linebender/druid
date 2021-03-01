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

//! Convenience methods for widgets.

use super::invalidation::DebugInvalidation;
use super::{
    Added, Align, BackgroundBrush, Click, Container, Controller, ControllerHost, EnvScope,
    IdentityWrapper, LensWrap, Padding, Parse, SizedBox, WidgetId,
};
use crate::widget::Scroll;
use crate::{
    Color, Data, Env, EventCtx, Insets, KeyOrValue, Lens, LifeCycleCtx, UnitPoint, Widget,
};

/// A trait that provides extra methods for combining `Widget`s.
pub trait WidgetExt<T: Data>: Widget<T> + Sized + 'static {
    /// Wrap this widget in a [`Padding`] widget with the given [`Insets`].
    ///
    /// [`Padding`]: widget/struct.Padding.html
    /// [`Insets`]: kurbo/struct.Insets.html
    fn padding(self, insets: impl Into<Insets>) -> Padding<T, Self> {
        Padding::new(insets, self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to center it.
    ///
    /// [`Align`]: widget/struct.Align.html
    fn center(self) -> Align<T> {
        Align::centered(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align left.
    ///
    /// [`Align`]: widget/struct.Align.html
    fn align_left(self) -> Align<T> {
        Align::left(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align right.
    ///
    /// [`Align`]: widget/struct.Align.html
    fn align_right(self) -> Align<T> {
        Align::right(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align vertically.
    ///
    /// [`Align`]: widget/struct.Align.html
    fn align_vertical(self, align: UnitPoint) -> Align<T> {
        Align::vertical(align, self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align horizontally.
    ///
    /// [`Align`]: widget/struct.Align.html
    fn align_horizontal(self, align: UnitPoint) -> Align<T> {
        Align::horizontal(align, self)
    }

    /// Wrap this widget in a [`SizedBox`] with an explicit width.
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn fix_width(self, width: f64) -> SizedBox<T> {
        SizedBox::new(self).width(width)
    }

    /// Wrap this widget in a [`SizedBox`] with an explicit height.
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn fix_height(self, height: f64) -> SizedBox<T> {
        SizedBox::new(self).height(height)
    }

    /// Wrap this widget in an [`SizedBox`] with an explicit width and height
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn fix_size(self, width: f64, height: f64) -> SizedBox<T> {
        SizedBox::new(self).width(width).height(height)
    }

    /// Wrap this widget in a [`SizedBox`] with an infinite width and height.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: #method.expand_height
    /// [`expand_width`]: #method.expand_width
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn expand(self) -> SizedBox<T> {
        SizedBox::new(self).expand()
    }

    /// Wrap this widget in a [`SizedBox`] with an infinite width.
    ///
    /// This will force the child to use all available space on the x-axis.
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn expand_width(self) -> SizedBox<T> {
        SizedBox::new(self).expand_width()
    }

    /// Wrap this widget in a [`SizedBox`] with an infinite width.
    ///
    /// This will force the child to use all available space on the y-axis.
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn expand_height(self) -> SizedBox<T> {
        SizedBox::new(self).expand_height()
    }

    /// Wrap this widget in a [`Container`] with the provided `background`.
    ///
    /// See [`Container::background`] for more information.
    ///
    /// [`Container`]: widget/struct.Container.html
    /// [`Container::background`]: widget/struct.Container.html#method.background
    fn background(self, brush: impl Into<BackgroundBrush<T>>) -> Container<T> {
        Container::new(self).background(brush)
    }

    /// Wrap this widget in a [`Container`] with the given border.
    ///
    /// Arguments can be either concrete values, or a [`Key`] of the respective
    /// type.
    ///
    /// [`Container`]: widget/struct.Container.html
    /// [`Key`]: struct.Key.html
    fn border(
        self,
        color: impl Into<KeyOrValue<Color>>,
        width: impl Into<KeyOrValue<f64>>,
    ) -> Container<T> {
        Container::new(self).border(color, width)
    }

    /// Wrap this widget in a [`EnvScope`] widget, modifying the parent
    /// [`Env`] with the provided closure.
    ///
    /// [`EnvScope`]: widget/struct.EnvScope.html
    /// [`Env`]: struct.Env.html
    fn env_scope(self, f: impl Fn(&mut Env, &T) + 'static) -> EnvScope<T, Self> {
        EnvScope::new(f, self)
    }

    /// Wrap this widget with the provided [`Controller`].
    ///
    /// [`Controller`]: widget/trait.Controller.html
    fn controller<C: Controller<T, Self>>(self, controller: C) -> ControllerHost<Self, C> {
        ControllerHost::new(self, controller)
    }

    /// Provide a closure that will be called when this widget is added to the widget tree.
    ///
    /// You can use this to perform any initial setup.
    ///
    /// This is equivalent to handling the [`LifeCycle::WidgetAdded`] event in a
    /// custom [`Controller`].
    ///
    /// [`LifeCycle::WidgetAdded`]: crate::LifeCycle::WidgetAdded
    fn on_added(
        self,
        f: impl Fn(&mut Self, &mut LifeCycleCtx, &T, &Env) + 'static,
    ) -> ControllerHost<Self, Added<T, Self>> {
        ControllerHost::new(self, Added::new(f))
    }

    /// Control the events of this widget with a [`Click`] widget. The closure
    /// provided will be called when the widget is clicked with the left mouse
    /// button.
    ///
    /// The child widget will also be updated on [`LifeCycle::HotChanged`] and
    /// mouse down, which can be useful for painting based on `ctx.is_active()`
    /// and `ctx.is_hot()`.
    ///
    /// [`Click`]: widget/struct.Click.html
    /// [`LifeCycle::HotChanged`]: enum.LifeCycle.html#variant.HotChanged
    fn on_click(
        self,
        f: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, Click<T>> {
        ControllerHost::new(self, Click::new(f))
    }

    /// Draw the [`layout`] `Rect`s of  this widget and its children.
    ///
    /// [`layout`]: trait.Widget.html#tymethod.layout
    fn debug_paint_layout(self) -> EnvScope<T, Self> {
        EnvScope::new(|env, _| env.set(Env::DEBUG_PAINT, true), self)
    }

    /// Display the `WidgetId`s for this widget and its children, when hot.
    ///
    /// When this is `true`, widgets that are `hot` (are under the mouse cursor)
    /// will display their ids in their bottom right corner.
    ///
    /// These ids may overlap; in this case the id of a child will obscure
    /// the id of its parent.
    fn debug_widget_id(self) -> EnvScope<T, Self> {
        EnvScope::new(|env, _| env.set(Env::DEBUG_WIDGET_ID, true), self)
    }

    /// Draw a color-changing rectangle over this widget, allowing you to see the
    /// invalidation regions.
    fn debug_invalidation(self) -> DebugInvalidation<T, Self> {
        DebugInvalidation::new(self)
    }

    /// Set the [`DEBUG_WIDGET`] env variable for this widget (and its descendants).
    ///
    /// This does nothing by default, but you can use this variable while
    /// debugging to only print messages from particular instances of a widget.
    ///
    /// [`DEBUG_WIDGET`]: struct.Env.html#associatedconstant.DEBUG_WIDGET
    fn debug_widget(self) -> EnvScope<T, Self> {
        EnvScope::new(|env, _| env.set(Env::DEBUG_WIDGET, true), self)
    }

    /// Wrap this widget in a [`LensWrap`] widget for the provided [`Lens`].
    ///
    ///
    /// [`LensWrap`]: struct.LensWrap.html
    /// [`Lens`]: trait.Lens.html
    fn lens<S: Data, L: Lens<S, T>>(self, lens: L) -> LensWrap<S, T, L, Self> {
        LensWrap::new(self, lens)
    }

    /// Parse a `Widget<String>`'s contents
    #[deprecated(since = "0.7.0", note = "Use TextBox::with_formatter instead")]
    fn parse(self) -> Parse<Self>
    where
        Self: Widget<String>,
    {
        Parse::new(self)
    }

    /// Assign the widget a specific [`WidgetId`].
    ///
    /// You must ensure that a given [`WidgetId`] is only ever used for
    /// a single widget at a time.
    ///
    /// An id _may_ be reused over time; for instance if you replace one
    /// widget with another, you may reuse the first widget's id.
    ///
    /// [`WidgetId`]: struct.WidgetId.html
    fn with_id(self, id: WidgetId) -> IdentityWrapper<Self> {
        IdentityWrapper::wrap(self, id)
    }

    /// Wrap this widget in a `Box`.
    fn boxed(self) -> Box<dyn Widget<T>> {
        Box::new(self)
    }

    /// Wrap this widget in a [`Scroll`] widget.
    ///
    /// [`Scroll`]: widget/struct.Scroll.html
    fn scroll(self) -> Scroll<T, Self> {
        Scroll::new(self)
    }
}

impl<T: Data, W: Widget<T> + 'static> WidgetExt<T> for W {}

// these are 'soft overrides' of methods on WidgetExt; resolution
// will choose an impl on a type over an impl in a trait for methods with the same
// name.

#[doc(hidden)]
impl<T: Data> SizedBox<T> {
    pub fn fix_width(self, width: f64) -> SizedBox<T> {
        self.width(width)
    }

    pub fn fix_height(self, height: f64) -> SizedBox<T> {
        self.height(height)
    }
}

// if two things are modifying an env one after another, just combine the modifications
#[doc(hidden)]
impl<T: Data, W> EnvScope<T, W> {
    pub fn env_scope(self, f2: impl Fn(&mut Env, &T) + 'static) -> EnvScope<T, W> {
        let EnvScope { f, child } = self;
        let new_f = move |env: &mut Env, data: &T| {
            f(env, data);
            f2(env, data);
        };
        EnvScope {
            f: Box::new(new_f),
            child,
        }
    }

    pub fn debug_paint_layout(self) -> EnvScope<T, W> {
        self.env_scope(|env, _| env.set(Env::DEBUG_PAINT, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::Slider;
    use crate::Color;
    use test_env_log::test;

    #[test]
    fn container_reuse() {
        // this should be Container<Align<Container<Slider>>>
        let widget = Slider::new()
            .background(Color::BLACK)
            .align_left()
            .border(Color::BLACK, 1.0);
        assert!(widget.border_is_some());
        assert!(!widget.background_is_some());

        // this should be Container<Slider>
        let widget = Slider::new()
            .background(Color::BLACK)
            .border(Color::BLACK, 1.0);
        assert!(widget.background_is_some());
        assert!(widget.border_is_some());
    }

    #[test]
    fn sized_box_reuse() {
        // this should be SizedBox<Align<SizedBox<Slider>>>
        let widget = Slider::new().fix_height(10.0).align_left().fix_width(1.0);
        assert_eq!(widget.width_and_height(), (Some(1.0), None));

        // this should be SizedBox<Slider>
        let widget = Slider::new().fix_height(10.0).fix_width(1.0);
        assert_eq!(widget.width_and_height(), (Some(1.0), Some(10.0)));
    }

    /// we only care that this will compile; see
    /// https://github.com/linebender/druid/pull/1414/
    #[test]
    fn lens_with_generic_param() {
        use crate::widget::{Checkbox, Flex, Slider};

        #[derive(Debug, Clone, Data, Lens)]
        struct MyData<T> {
            data: T,
            floatl: f64,
        }

        #[allow(dead_code)]
        fn make_widget() -> impl Widget<MyData<bool>> {
            Flex::row()
                .with_child(Slider::new().lens(MyData::<bool>::floatl))
                .with_child(Checkbox::new("checkbox").lens(MyData::<bool>::data))
        }
    }
}
