use crate::kurbo::{Affine, Point, Rect, Size};
use crate::piet::RenderContext;
use crate::widget::WidgetHolder;
use crate::{BoxConstraints, EventCtx, LayoutCtx, MouseEvent, PaintCtx, Widget};

/// Manages the position of a child widget.
pub struct LayoutHost<W> {
    size: Size,
    debug_needs_set_origin: bool,

    origin: Point,
    child: W,
}

impl<W> LayoutHost<W> {
    /// Create a new `LayoutHost` for the given child.
    ///
    /// After this widget is laid out, the parent *must* call
    /// [`LayoutHost::set_origin`] in order to set the child's position.
    pub fn new(child: W) -> Self {
        LayoutHost {
            child,
            size: Size::ZERO,
            origin: Point::ZERO,
            debug_needs_set_origin: true,
        }
    }

    /// Set the position of the child, relative to the origin of the parent.
    pub fn set_origin(&mut self, point: Point) {
        self.origin = point;
        self.debug_needs_set_origin = false;
    }

    /// The child's size.
    pub fn size(&self) -> Size {
        self.size
    }

    fn contains(&self, mouse: &MouseEvent) -> bool {
        Rect::from_origin_size(self.origin, self.size).contains(mouse.pos)
    }

    fn propagate_mouse_if_needed(
        &mut self,
        ctx: &mut EventCtx,
        event: &MouseEvent,
        f: impl FnOnce(&mut W, &mut EventCtx, &MouseEvent),
    ) {
        let mut mouse = event.clone();
        mouse.pos -= self.origin.to_vec2();
        let hovered = self.contains(&mouse);
        let pre_hovered = ctx.state.hovered;
        ctx.state.hovered = hovered;
        if ctx.state.should_receive_mouse() {
            f(&mut self.child, ctx, &mouse);
        }
        ctx.state.hovered = pre_hovered;
    }
}

impl<W: Widget> WidgetHolder for LayoutHost<W> {
    type Child = W;

    fn widget(&self) -> &Self::Child {
        &self.child
    }

    fn widget_mut(&mut self) -> &mut Self::Child {
        &mut self.child
    }
    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.propagate_mouse_if_needed(ctx, event, |child, ctx, e| child.mouse_down(ctx, e));
    }

    fn mouse_move(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.propagate_mouse_if_needed(ctx, event, |child, ctx, e| child.mouse_down(ctx, e));
    }

    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.propagate_mouse_if_needed(ctx, event, |child, ctx, e| child.mouse_down(ctx, e));
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        self.debug_needs_set_origin = true;
        self.size = self.child.layout(ctx, bc);
        //TODO: validate that size matches constraints?
        self.size
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        if self.debug_needs_set_origin {
            panic!("Missing call to set_origin");
        }
        ctx.with_save(|ctx| {
            let layout_origin = self.origin.to_vec2();
            ctx.transform(Affine::translate(layout_origin));
            self.child.paint(ctx);
        });
    }
}
