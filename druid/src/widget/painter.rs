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

use crate::piet::{FixedGradient, LinearGradient, PaintBrush, RadialGradient};
use crate::{
    BoxConstraints, Color, Data, Env, Event, EventCtx, Key, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, RenderContext, Size, UpdateCtx, Widget,
};

/// A widget that only handles painting.
///
/// This is useful in a situation where layout is controlled elsewhere and you
/// do not need to handle events, but you would like to customize appearance.
///
/// **When is paint called?**
///
/// The `Painter` widget will call its [`paint`]  method anytime its [`Data`]
/// is changed. If you would like it to repaint at other times (such as when
/// hot or active state changes) you will need to call [`request_paint`] further
/// up the tree, perhaps in a [`Controller`] widget.
///
/// # Examples
///
/// Changing background color based on some part of data:
///
/// ```
/// use druid::{Env, PaintCtx,Rect, RenderContext};
/// use druid::widget::Painter;
/// # const ENABLED_BG_COLOR: druid::Key<druid::Color> = druid::Key::new("fake key");
/// # const DISABLED_BG_COLOR: druid::Key<druid::Color> = druid::Key::new("fake key 2");
///
/// struct MyData { is_enabled: bool }
///
/// let my_painter = Painter::new(|ctx, data: &MyData, env| {
///     let bounds = ctx.size().to_rect();
///     if data.is_enabled {
///         ctx.fill(bounds, &env.get(ENABLED_BG_COLOR));
///     } else {
///
///         ctx.fill(bounds, &env.get(DISABLED_BG_COLOR));
///     }
/// });
/// ```
///
/// Using painter to make a simple widget that will draw a selected color
///
///
/// ```
/// use druid::{Color, Env, PaintCtx,Rect, RenderContext};
/// use druid::widget::Painter;
///
/// const CORNER_RADIUS: f64 = 4.0;
/// const STROKE_WIDTH: f64 = 2.0;
///
/// let colorwell: Painter<Color> = Painter::new(|ctx, data: &Color, env| {
///     // Shrink the bounds a little, to ensure that our stroke remains within
///     // the paint bounds.
///     let bounds = ctx.size().to_rect().inset(-STROKE_WIDTH / 2.0);
///     let rounded = bounds.to_rounded_rect(CORNER_RADIUS);
///     ctx.fill(rounded, data);
///     ctx.stroke(rounded, &env.get(druid::theme::PRIMARY_DARK), STROKE_WIDTH);
/// });
/// ```
///
/// [`paint`]: ../trait.Widget.html#tymethod.paint
/// [`Data`]: ../trait.Data.html
/// [`request_paint`]: ../EventCtx.html#method.request_paint
/// [`Controller`]: trait.Controller.html
pub struct Painter<T>(Box<dyn FnMut(&mut PaintCtx, &T, &Env)>);

/// Something that can be used as the background for a widget.
///
/// This represents anything that can be painted inside a widgets [`paint`]
/// method; that is, it may have access to the [`Data`] and the [`Env`].
///
/// [`paint`]: ../trait.Widget.html#tymethod.paint
/// [`Data`]: ../trait.Data.html
/// [`Env`]: ../struct.Env.html
#[non_exhaustive]
#[allow(missing_docs)]
pub enum BackgroundBrush<T> {
    Color(Color),
    ColorKey(Key<Color>),
    Linear(LinearGradient),
    Radial(RadialGradient),
    Fixed(FixedGradient),
    Painter(Painter<T>),
}

impl<T> Painter<T> {
    /// Create a new `Painter` with the provided [`paint`] fn.
    ///
    /// [`paint`]: ../trait.Widget.html#tymethod.paint
    pub fn new(f: impl FnMut(&mut PaintCtx, &T, &Env) + 'static) -> Self {
        Painter(Box::new(f))
    }
}

impl<T: Data> BackgroundBrush<T> {
    /// Draw this `BackgroundBrush` into a provided [`PaintCtx`].
    ///
    /// [`PaintCtx`]: ../struct.PaintCtx.html
    pub fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let bounds = ctx.size().to_rect();
        match self {
            Self::Color(color) => ctx.fill(bounds, color),
            Self::ColorKey(key) => ctx.fill(bounds, &env.get(key)),
            Self::Linear(grad) => ctx.fill(bounds, grad),
            Self::Radial(grad) => ctx.fill(bounds, grad),
            Self::Fixed(grad) => ctx.fill(bounds, grad),
            Self::Painter(painter) => painter.paint(ctx, data, env),
        }
    }
}

impl<T: Data> Widget<T> for Painter<T> {
    fn event(&mut self, _: &mut EventCtx, _: &Event, _: &mut T, _: &Env) {}
    fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &T, _: &Env) {}
    fn update(&mut self, ctx: &mut UpdateCtx, old: &T, new: &T, _: &Env) {
        if !old.same(new) {
            ctx.request_paint();
        }
    }
    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _: &T, _: &Env) -> Size {
        bc.max()
    }
    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        (self.0)(ctx, data, env)
    }
}

impl<T> From<Color> for BackgroundBrush<T> {
    fn from(src: Color) -> BackgroundBrush<T> {
        BackgroundBrush::Color(src)
    }
}

impl<T> From<Key<Color>> for BackgroundBrush<T> {
    fn from(src: Key<Color>) -> BackgroundBrush<T> {
        BackgroundBrush::ColorKey(src)
    }
}

impl<T> From<LinearGradient> for BackgroundBrush<T> {
    fn from(src: LinearGradient) -> BackgroundBrush<T> {
        BackgroundBrush::Linear(src)
    }
}

impl<T> From<RadialGradient> for BackgroundBrush<T> {
    fn from(src: RadialGradient) -> BackgroundBrush<T> {
        BackgroundBrush::Radial(src)
    }
}

impl<T> From<FixedGradient> for BackgroundBrush<T> {
    fn from(src: FixedGradient) -> BackgroundBrush<T> {
        BackgroundBrush::Fixed(src)
    }
}

impl<T> From<Painter<T>> for BackgroundBrush<T> {
    fn from(src: Painter<T>) -> BackgroundBrush<T> {
        BackgroundBrush::Painter(src)
    }
}

impl<T> From<PaintBrush> for BackgroundBrush<T> {
    fn from(src: PaintBrush) -> BackgroundBrush<T> {
        match src {
            PaintBrush::Linear(grad) => BackgroundBrush::Linear(grad),
            PaintBrush::Radial(grad) => BackgroundBrush::Radial(grad),
            PaintBrush::Fixed(grad) => BackgroundBrush::Fixed(grad),
            PaintBrush::Color(color) => BackgroundBrush::Color(color),
        }
    }
}
