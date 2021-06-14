use crate::piet::Color;
use crate::widget::Never;
use crate::widgets::{
    layout::{Align, SizedBox},
    ActionMapper, Background,
};
use crate::{EventCtx, Widget};

pub trait WidgetExt: Sized {
    /// Wrap this widget in a [`SizedBox`] with an explicit width.
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn fix_width(self, width: f64) -> SizedBox<Self> {
        SizedBox::new(self).width(width)
    }

    /// Wrap this widget in a [`SizedBox`] with an explicit height.
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn fix_height(self, height: f64) -> SizedBox<Self> {
        SizedBox::new(self).height(height)
    }

    /// Wrap this widget in an [`SizedBox`] with an explicit width and height
    ///
    /// [`SizedBox`]: widget/struct.SizedBox.html
    fn fix_size(self, width: f64, height: f64) -> SizedBox<Self> {
        SizedBox::new(self).width(width).height(height)
    }

    /// Wrap this widget in an [`Align`] widget, configured to center it.
    ///
    /// [`Align`]: widget/struct.Align.html
    fn center(self) -> Align<Self> {
        Align::new(self).centered()
    }

    fn background(self, color: Color) -> Background<Self> {
        Background::new(self).background(color)
    }

    fn border(self, color: Color, width: f64) -> Background<Self> {
        Background::new(self).border(color, width)
    }

    fn map_actions<In, Out>(
        self,
        mut map: impl FnMut(In) -> Out + 'static,
    ) -> ActionMapper<Self, In, Out> {
        ActionMapper::new(self, move |x, _| Some(map(x)))
    }

    fn filter_map_actions<In, Out>(
        self,
        mut map: impl FnMut(In) -> Option<Out> + 'static,
    ) -> ActionMapper<Self, In, Out> {
        ActionMapper::new(self, move |x, _| map(x))
    }

    fn suppress_actions<In, Out>(self) -> ActionMapper<Self, In, Out> {
        ActionMapper::new(self, |_, _| None)
    }

    /// Handle all actions.
    fn handle_actions<In>(
        self,
        mut f: impl FnMut(In, &mut EventCtx<Never>) + 'static,
    ) -> ActionMapper<Self, In, Never> {
        ActionMapper::new(self, move |x, ctx| {
            f(x, ctx);
            None
        })
    }
}

impl<W: Widget> WidgetExt for W {}
