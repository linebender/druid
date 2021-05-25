use crate::kurbo::Size;
use crate::{BoxConstraints, EventCtx, LayoutCtx, MouseEvent, PaintCtx};
use druid_shell::{KeyEvent, TimerToken};

#[allow(unused_variables)]
pub trait Widget {
    fn init(&mut self, ctx: &mut EventCtx) {}
    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {}
    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {}
    fn mouse_move(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {}
    fn scroll(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {}
    fn key_down(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {}
    fn key_up(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {}
    fn timer(&mut self, ctx: &mut EventCtx, token: TimerToken) {}
    //fn idle(&mut self, ctx: &mut EventCtx, token: TimerToken) {}
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        Size::ZERO
    }
    fn update(&mut self) {}
    fn paint(&self, ctx: &mut PaintCtx) {}
}

/// The null widget, which does nothing.
impl Widget for () {
    // not quite nothing; it respects its constraints.
    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        bc.constrain(Size::ZERO)
    }
}

/// A helper trait for types that wrap a single widget.
///
/// This provides default implementations of widget methods that forward to
/// the inner widget; the wrapping widget only needs to override the specific
/// methods it is interested in.
pub trait SingleChildContainer {
    type Child: Widget;

    fn widget(&self) -> &Self::Child;

    fn widget_mut(&mut self) -> &mut Self::Child;

    fn init(&mut self, ctx: &mut EventCtx) {
        self.widget_mut().init(ctx)
    }
    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.widget_mut().mouse_down(ctx, event)
    }
    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.widget_mut().mouse_up(ctx, event)
    }
    fn mouse_move(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.widget_mut().mouse_move(ctx, event)
    }
    fn scroll(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.widget_mut().scroll(ctx, event)
    }
    fn key_down(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        self.widget_mut().key_down(ctx, event)
    }
    fn key_up(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        self.widget_mut().key_up(ctx, event)
    }
    fn timer(&mut self, ctx: &mut EventCtx, token: TimerToken) {
        self.widget_mut().timer(ctx, token)
    }
    fn update(&mut self) {
        self.widget_mut().update()
    }
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        self.widget_mut().layout(ctx, bc)
    }
    fn paint(&self, ctx: &mut PaintCtx) {
        self.widget().paint(ctx)
    }
}

impl<T: SingleChildContainer<Child = W>, W: Widget> Widget for T {
    fn init(&mut self, ctx: &mut EventCtx) {
        <Self as SingleChildContainer>::init(self, ctx)
    }
    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        <Self as SingleChildContainer>::mouse_down(self, ctx, event)
    }
    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        <Self as SingleChildContainer>::mouse_up(self, ctx, event)
    }
    fn mouse_move(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        <Self as SingleChildContainer>::mouse_move(self, ctx, event)
    }
    fn scroll(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        <Self as SingleChildContainer>::scroll(self, ctx, event)
    }
    fn key_down(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        <Self as SingleChildContainer>::key_down(self, ctx, event)
    }
    fn key_up(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        <Self as SingleChildContainer>::key_up(self, ctx, event)
    }
    fn timer(&mut self, ctx: &mut EventCtx, token: TimerToken) {
        <Self as SingleChildContainer>::timer(self, ctx, token)
    }
    fn update(&mut self) {
        <Self as SingleChildContainer>::update(self)
    }
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        <Self as SingleChildContainer>::layout(self, ctx, bc)
    }
    fn paint(&self, ctx: &mut PaintCtx) {
        <Self as SingleChildContainer>::paint(self, ctx)
    }
}
