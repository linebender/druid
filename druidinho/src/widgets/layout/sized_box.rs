use crate::kurbo::Size;
use crate::widget::WidgetHolder;
use crate::{BoxConstraints, LayoutCtx, Widget};

#[derive(Debug, Default)]
pub struct SizedBox<W> {
    inner: W,
    width: Option<f64>,
    height: Option<f64>,
}

impl<W> SizedBox<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            width: None,
            height: None,
        }
    }

    pub fn empty() -> SizedBox<()> {
        SizedBox::default()
    }

    /// Set container's width.
    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Set container's height.
    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.height = Some(size.height);
        self.width = Some(size.width);
        self
    }

    /// Expand container to fit the parent.
    ///
    /// Only call this method if you want your widget to occupy all available
    /// space. If you only care about expanding in one of width or height, use
    /// [`expand_width`] or [`expand_height`] instead.
    ///
    /// [`expand_height`]: #method.expand_height
    /// [`expand_width`]: #method.expand_width
    pub fn expand(mut self) -> Self {
        self.width = Some(f64::INFINITY);
        self.height = Some(f64::INFINITY);
        self
    }

    /// Expand the container on the x-axis.
    ///
    /// This will force the child to have maximum width.
    pub fn expand_width(mut self) -> Self {
        self.width = Some(f64::INFINITY);
        self
    }

    /// Expand the container on the y-axis.
    ///
    /// This will force the child to have maximum height.
    pub fn expand_height(mut self) -> Self {
        self.height = Some(f64::INFINITY);
        self
    }

    fn child_constraints(&self, bc: &BoxConstraints) -> BoxConstraints {
        // if we don't have a width/height, we don't change that axis.
        // if we have a width/height, we clamp it on that axis.
        let (min_width, max_width) = match self.width {
            Some(width) => {
                let w = width.max(bc.min().width).min(bc.max().width);
                (w, w)
            }
            None => (bc.min().width, bc.max().width),
        };

        let (min_height, max_height) = match self.height {
            Some(height) => {
                let h = height.max(bc.min().height).min(bc.max().height);
                (h, h)
            }
            None => (bc.min().height, bc.max().height),
        };

        BoxConstraints::new(
            Size::new(min_width, min_height),
            Size::new(max_width, max_height),
        )
    }
}

impl<W: Widget> WidgetHolder for SizedBox<W> {
    type Child = W;

    fn widget(&self) -> &Self::Child {
        &self.inner
    }

    fn widget_mut(&mut self) -> &mut Self::Child {
        &mut self.inner
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        bc.debug_check("SizedBox");
        let child_bc = self.child_constraints(&bc);
        let size = bc.constrain(self.inner.layout(ctx, child_bc));

        if size.width.is_infinite() {
            eprintln!("SizedBox is returning an infinite width.");
        }

        if size.height.is_infinite() {
            eprintln!("SizedBox is returning an infinite height.");
        }
        size
    }
}
