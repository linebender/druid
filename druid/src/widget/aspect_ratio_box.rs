use druid::widget::prelude::*;
use druid::Data;
use tracing::warn;

/// A widget that preserves the aspect ratio given to it.
///
/// If given a child, this widget forces the child to have a width and height that preserves
/// the aspect ratio.
///
/// If not given a child, The box will try to size itself  as large or small as possible
/// to preserve the aspect ratio.
pub struct AspectRatioBox<T> {
    inner: Option<Box<dyn Widget<T>>>,
    ratio: f64,
}

impl<T> AspectRatioBox<T> {
    /// Create container with a child and aspect ratio.
    ///
    /// If aspect ratio <= 0.0, the ratio will be set to 1.0
    pub fn new(inner: impl Widget<T> + 'static, ratio: f64) -> Self {
        Self {
            inner: Some(Box::new(inner)),
            ratio: AspectRatioBox::<T>::clamp_ratio(ratio),
        }
    }

    /// Create container without child but with an aspect ratio.
    ///
    /// If aspect ratio <= 0.0, the ratio will be set to 1.0
    pub fn empty(ratio: f64) -> Self {
        Self {
            inner: None,
            ratio: AspectRatioBox::<T>::clamp_ratio(ratio),
        }
    }

    /// Set the ratio of the box.
    ///
    /// The ratio has to be a value between 0 and 1, excluding 0. It will be clamped
    /// to those values if they exceed the bounds. If the ratio is 0, then the ratio
    /// will become 1.
    pub fn set_ratio(&mut self, ratio: f64) {
        self.ratio = AspectRatioBox::<T>::clamp_ratio(ratio);
    }

    // clamps the ratio between 0.0 and f64::MAX
    // if ratio is 0.0 then it will return 1.0 to avoid creating NaN
    fn clamp_ratio(mut ratio: f64) -> f64 {
        ratio = f64::clamp(ratio, 0.0, f64::MAX);
        if ratio == 0.0 {
            // should I force the ratio to be 1 in this case?
            1.0
        } else {
            ratio
        }
    }
}

impl<T: Data> Widget<T> for AspectRatioBox<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.lifecycle(ctx, event, data, env)
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.update(ctx, old_data, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        bc.debug_check("AspectRatioBox");

        let bc = if bc.max() == bc.min() {
            warn!("Box constraints are tight. Aspect ratio box will not be able to preserve aspect ratio.");

            *bc
        } else if bc.max().width == f64::INFINITY && bc.max().height == f64::INFINITY {
            warn!("Box constraints are INFINITE. Aspect ratio box won't be able to choose a size because the constraints given by the parent widget are INFINITE.");

            // should I do this or should I just choose a default size
            // I size the box with the child's size if there is one
            let size = match self.inner.as_mut() {
                Some(inner) => inner.layout(ctx, &bc, data, env),
                None => Size::new(500., 500.),
            };

            BoxConstraints::new(Size::new(0., 0.), size)
        } else {
            let (mut box_width, mut box_height) = (bc.max().width, bc.max().height);

            if self.ratio < 1.0 {
                if (box_height >= box_width && box_height * self.ratio <= box_width)
                    || box_width > box_height
                {
                    box_width = box_height * self.ratio;
                } else if box_height >= box_width && box_height * self.ratio > box_width {
                    box_height = box_width / self.ratio;
                } else {
                    // I'm not sure if these sections are necessary or correct
                    // dbg!("ratio can't be preserved {}", self.ratio);
                    warn!("The aspect ratio cannot be preserved because one of dimensions is tight and the other dimension is too small: bc.max() = {}, bc.min() = {}", bc.max(), bc.min());
                }

                BoxConstraints::tight(Size::new(box_width, box_height))
            } else if self.ratio > 1.0 {
                if box_width > box_height && box_height * self.ratio <= box_width {
                    box_width = box_height * self.ratio;
                }
                // this condition might be wrong if box_width and height are equal to each other
                // and the aspect ratio is something like 1.00000000000000001, in this case
                // the  box_height * self.ratio could be equal to box_width
                // however 1.00000000000000001 does equal 1.0
                else if (box_width >= box_height && box_height * self.ratio > box_width)
                    || box_height > box_width
                {
                    box_height = box_width / self.ratio;
                } else {
                    // I'm not sure if these sections are necessary or correct
                    // dbg!("ratio can't be preserved {}", self.ratio);
                    warn!("The aspect ratio cannot be preserved because one of dimensions is tight and the other dimension is too small: bc.max() = {}, bc.min() = {}", bc.max(), bc.min());
                }

                BoxConstraints::tight(Size::new(box_width, box_height))
            }
            // the aspect ratio is 1:1 which means we want a square
            // we take the minimum between the width and height and constrain to that min
            else {
                let min = box_width.min(box_height);
                BoxConstraints::tight(Size::new(min, min))
            }
        };

        let size = match self.inner.as_mut() {
            Some(inner) => inner.layout(ctx, &bc, data, env),
            None => bc.max(),
        };

        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if let Some(ref mut inner) = self.inner {
            inner.paint(ctx, data, env);
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.as_ref().and_then(|inner| inner.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::harness::*;
    use crate::widget::Label;
    use crate::WidgetExt;

    #[test]
    fn tight_constraints() {
        let id = WidgetId::next();
        let (width, height) = (400., 400.);
        let aspect = AspectRatioBox::<()>::new(Label::new("hello!"), 1.0)
            .with_id(id)
            .fix_width(width)
            .fix_height(height)
            .center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), aspect, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(width, height));
        });
    }

    #[test]
    fn infinite_constraints_with_child() {
        let id = WidgetId::next();
        let (width, height) = (100., 100.);
        let label = Label::new("hello!").fix_width(width).height(height);
        let aspect = AspectRatioBox::<()>::new(label, 1.0)
            .with_id(id)
            .scroll()
            .center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), aspect, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(width, height));
        });
    }
    #[test]
    fn infinite_constraints_without_child() {
        let id = WidgetId::next();
        let aspect = AspectRatioBox::<()>::empty(1.0)
            .with_id(id)
            .scroll()
            .center();

        let (window_width, window_height) = (600., 600.);

        Harness::create_simple((), aspect, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            assert_eq!(state.layout_rect().size(), Size::new(500., 500.));
        });
    }

    // this test still needs some work
    // I am testing for this condition:
    // The box constraint on the width's min and max is 300.0.
    // The height of the window is 50.0 and width 600.0.
    // I'm not sure what size the SizedBox passes in for the height constraint
    // but it is most likely 50.0 for max and 0.0 for min.
    // The aspect ratio is 2.0 which means the box has to have dimensions (300., 150.)
    // however given these constraints it isn't possible.
    // should the aspect ratio box maintain aspect ratio anyways or should it clip/overflow?
    #[test]
    fn tight_constraint_on_width() {
        let id = WidgetId::next();
        let label = Label::new("hello!");
        let aspect = AspectRatioBox::<()>::new(label, 2.0)
            .with_id(id)
            .fix_width(300.)
            // wrap in align widget because root widget must fill the window space
            .center();

        let (window_width, window_height) = (600., 50.);

        Harness::create_simple((), aspect, |harness| {
            harness.set_initial_size(Size::new(window_width, window_height));
            harness.send_initial_events();
            harness.just_layout();
            let state = harness.get_state(id);
            dbg!(state.layout_rect().size());
            // assert_eq!(state.layout_rect().size(), Size::new(500., 500.));
        });
    }
}
