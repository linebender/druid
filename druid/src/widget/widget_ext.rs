// Copyright 2019 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Convenience methods for widgets.

use super::invalidation::DebugInvalidation;
#[allow(deprecated)]
use super::Parse;
use super::{
    Added, Align, BackgroundBrush, Click, Container, Controller, ControllerHost, EnvScope,
    IdentityWrapper, LensWrap, Padding, SizedBox, WidgetId,
};
use crate::widget::{DisabledIf, Scroll};
use crate::{
    Color, Data, Env, EventCtx, Insets, KeyOrValue, Lens, LifeCycleCtx, UnitPoint, Widget,
};

/// A trait that provides extra methods for combining `Widget`s.
pub trait WidgetExt<T: Data>: Widget<T> + Sized + 'static {
    /// Wrap this widget in a [`Padding`] widget with the given [`Insets`].
    ///
    /// Like [`Padding::new`], this can accept a variety of arguments, including
    /// a [`Key`] referring to [`Insets`] in the [`Env`].
    ///
    /// [`Key`]: crate::Key
    fn padding(self, insets: impl Into<KeyOrValue<Insets>>) -> Padding<T, Self> {
        Padding::new(insets, self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to center it.
    fn center(self) -> Align<T> {
        Align::centered(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align left.
    fn align_left(self) -> Align<T> {
        Align::left(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align right.
    fn align_right(self) -> Align<T> {
        Align::right(self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align vertically.
    fn align_vertical(self, align: UnitPoint) -> Align<T> {
        Align::vertical(align, self)
    }

    /// Wrap this widget in an [`Align`] widget, configured to align horizontally.
    fn align_horizontal(self, align: UnitPoint) -> Align<T> {
        Align::horizontal(align, self)
    }

    /// Wrap this widget in a [`SizedBox`] with an explicit width.
    fn fix_width(self, width: impl Into<KeyOrValue<f64>>) -> SizedBox<T> {
        SizedBox::new(self).width(width)
    }

    /// Wrap this widget in a [`SizedBox`] with an explicit height.
    fn fix_height(self, height: impl Into<KeyOrValue<f64>>) -> SizedBox<T> {
        SizedBox::new(self).height(height)
    }

    /// Wrap this widget in an [`SizedBox`] with an explicit width and height
    fn fix_size(
        self,
        width: impl Into<KeyOrValue<f64>>,
        height: impl Into<KeyOrValue<f64>>,
    ) -> SizedBox<T> {
        SizedBox::new(self).width(width).height(height)
    }

    /// Wrap this widget in a [`SizedBox`] with an infinite width and height.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: WidgetExt::expand_height
    /// [`expand_width`]: WidgetExt::expand_width
    fn expand(self) -> SizedBox<T> {
        SizedBox::new(self).expand()
    }

    /// Wrap this widget in a [`SizedBox`] with an infinite width.
    ///
    /// This will force the child to use all available space on the x-axis.
    fn expand_width(self) -> SizedBox<T> {
        SizedBox::new(self).expand_width()
    }

    /// Wrap this widget in a [`SizedBox`] with an infinite width.
    ///
    /// This will force the child to use all available space on the y-axis.
    fn expand_height(self) -> SizedBox<T> {
        SizedBox::new(self).expand_height()
    }

    /// Wrap this widget in a [`Container`] with the provided background `brush`.
    ///
    /// See [`Container::background`] for more information.
    fn background(self, brush: impl Into<BackgroundBrush<T>>) -> Container<T> {
        Container::new(self).background(brush)
    }

    /// Wrap this widget in a [`Container`] with the provided foreground `brush`.
    ///
    /// See [`Container::foreground`] for more information.
    fn foreground(self, brush: impl Into<BackgroundBrush<T>>) -> Container<T> {
        Container::new(self).foreground(brush)
    }

    /// Wrap this widget in a [`Container`] with the given border.
    ///
    /// Arguments can be either concrete values, or a [`Key`] of the respective
    /// type.
    ///
    /// [`Key`]: crate::Key
    fn border(
        self,
        color: impl Into<KeyOrValue<Color>>,
        width: impl Into<KeyOrValue<f64>>,
    ) -> Container<T> {
        Container::new(self).border(color, width)
    }

    /// Wrap this widget in a [`EnvScope`] widget, modifying the parent
    /// [`Env`] with the provided closure.
    fn env_scope(self, f: impl Fn(&mut Env, &T) + 'static) -> EnvScope<T, Self> {
        EnvScope::new(f, self)
    }

    /// Wrap this widget with the provided [`Controller`].
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
    /// [`LifeCycle::HotChanged`]: crate::LifeCycle::HotChanged
    fn on_click(
        self,
        f: impl Fn(&mut EventCtx, &mut T, &Env) + 'static,
    ) -> ControllerHost<Self, Click<T>> {
        ControllerHost::new(self, Click::new(f))
    }

    /// Draw the [`layout`] `Rect`s of  this widget and its children.
    ///
    /// [`layout`]: Widget::layout
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
    /// [`DEBUG_WIDGET`]: crate::Env::DEBUG_WIDGET
    fn debug_widget(self) -> EnvScope<T, Self> {
        EnvScope::new(|env, _| env.set(Env::DEBUG_WIDGET, true), self)
    }

    /// Wrap this widget in a [`LensWrap`] widget for the provided [`Lens`].
    fn lens<S: Data, L: Lens<S, T>>(self, lens: L) -> LensWrap<S, T, L, Self> {
        LensWrap::new(self, lens)
    }

    /// Parse a `Widget<String>`'s contents
    #[doc(hidden)]
    #[deprecated(since = "0.7.0", note = "Use TextBox::with_formatter instead")]
    #[allow(deprecated)]
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
    fn with_id(self, id: WidgetId) -> IdentityWrapper<Self> {
        IdentityWrapper::wrap(self, id)
    }

    /// Wrap this widget in a `Box`.
    fn boxed(self) -> Box<dyn Widget<T>> {
        Box::new(self)
    }

    /// Wrap this widget in a [`Scroll`] widget.
    fn scroll(self) -> Scroll<T, Self> {
        Scroll::new(self)
    }

    /// Wrap this widget in a [`DisabledIf`] widget.
    ///
    /// The provided closure will determine if the widget is disabled.
    /// See [`is_disabled`] or [`set_disabled`] for more info about disabled state.
    ///
    /// [`is_disabled`]: EventCtx::is_disabled
    /// [`set_disabled`]: EventCtx::set_disabled
    fn disabled_if(self, disabled_if: impl Fn(&T, &Env) -> bool + 'static) -> DisabledIf<T, Self> {
        DisabledIf::new(self, disabled_if)
    }
}

impl<T: Data, W: Widget<T> + 'static> WidgetExt<T> for W {}

// these are 'soft overrides' of methods on WidgetExt; resolution
// will choose an impl on a type over an impl in a trait for methods with the same
// name.

#[doc(hidden)]
impl<T: Data> SizedBox<T> {
    pub fn fix_width(self, width: impl Into<KeyOrValue<f64>>) -> SizedBox<T> {
        self.width(width)
    }

    pub fn fix_height(self, height: impl Into<KeyOrValue<f64>>) -> SizedBox<T> {
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
    use crate::{Color, Key};
    use test_log::test;

    #[test]
    fn container_reuse() {
        // this should be Container<Align<Container<Slider>>>
        let widget = Slider::new()
            .background(Color::BLACK)
            .foreground(Color::WHITE)
            .align_left()
            .border(Color::BLACK, 1.0);
        assert!(widget.border_is_some());
        assert!(!widget.background_is_some());
        assert!(!widget.foreground_is_some());

        // this should be Container<Slider>
        let widget = Slider::new()
            .background(Color::BLACK)
            .border(Color::BLACK, 1.0)
            .foreground(Color::WHITE);
        assert!(widget.background_is_some());
        assert!(widget.border_is_some());
        assert!(widget.foreground_is_some());
    }

    #[test]
    fn sized_box_reuse() {
        let mut env = Env::empty();

        // this should be SizedBox<Align<SizedBox<Slider>>>
        let widget = Slider::new().fix_height(10.0).align_left().fix_width(1.0);
        assert_eq!(widget.width_and_height(&env), (Some(1.0), None));

        // this should be SizedBox<Slider>
        let widget = Slider::new().fix_height(10.0).fix_width(1.0);
        assert_eq!(widget.width_and_height(&env), (Some(1.0), Some(10.0)));

        const HEIGHT_KEY: Key<f64> = Key::new("test-sized-box-reuse-height");
        const WIDTH_KEY: Key<f64> = Key::new("test-sized-box-reuse-width");
        env.set(HEIGHT_KEY, 10.0);
        env.set(WIDTH_KEY, 1.0);

        // this should be SizedBox<Align<SizedBox<Slider>>>
        let widget = Slider::new()
            .fix_height(HEIGHT_KEY)
            .align_left()
            .fix_width(WIDTH_KEY);
        assert_eq!(widget.width_and_height(&env), (Some(1.0), None));

        // this should be SizedBox<Slider>
        let widget = Slider::new().fix_height(HEIGHT_KEY).fix_width(WIDTH_KEY);
        assert_eq!(widget.width_and_height(&env), (Some(1.0), Some(10.0)));
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
