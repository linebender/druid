use crate::kurbo::Size;
use druid_shell::{KeyEvent, TimerToken};

use crate::contexts::{EventCtx, LayoutCtx, PaintCtx};
use crate::widgets::layout::LayoutHost;
use crate::{BoxConstraints, MouseEvent, Widget};

pub struct WidgetHost<W> {
    child: LayoutHost<W>,
    state: WidgetState,
}

pub(crate) struct WidgetState {
    /// The mouse is inside the widget's frame.
    pub(crate) hovered: bool,
    /// The widget has mouse focus.
    ///
    /// The widget will receive all mouse events until it releases focus.
    pub(crate) mouse_focus: bool,
    /// A descendent of this widget has captured the mouse
    child_mouse_focus: bool,
    /// The widget has keyboard focus
    pub(crate) keyboard_focus: bool,
    /// A descendent of the widget has keyboard focus
    child_keyboard_focus: bool,
}

impl<W: Widget> WidgetHost<W> {
    fn with_child<R>(
        &mut self,
        parent_ctx: &mut EventCtx,
        f: impl FnOnce(&mut LayoutHost<W>, &mut EventCtx) -> R,
    ) -> R {
        self.state.child_keyboard_focus = false;
        self.state.child_mouse_focus = false;

        let mut child_ctx = EventCtx {
            state: &mut self.state,
            window: parent_ctx.window,
        };
        let r = f(&mut self.child, &mut child_ctx);
        parent_ctx.state.merge_up(child_ctx.state);
        r
    }
}

impl<W: Widget> Widget for WidgetHost<W> {
    fn init(&mut self, ctx: &mut EventCtx) {
        self.with_child(ctx, |chld, ctx| chld.init(ctx))
    }
    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.mouse_down(ctx, event));
    }
    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.mouse_up(ctx, event));
    }

    fn mouse_move(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.mouse_move(ctx, event));
    }
    fn scroll(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.scroll(ctx, event));
    }
    fn key_down(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        if self.state.keyboard_focus || self.state.child_keyboard_focus {
            self.with_child(ctx, |chld, ctx| chld.key_down(ctx, event));
        }
    }

    fn key_up(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        if self.state.keyboard_focus || self.state.child_keyboard_focus {
            self.with_child(ctx, |chld, ctx| chld.key_up(ctx, event));
        }
    }

    fn timer(&mut self, ctx: &mut EventCtx, token: TimerToken) {
        self.with_child(ctx, |chld, ctx| chld.timer(ctx, token));
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        // we always lay out eveything
        self.child.layout(ctx, bc)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        // we always paint everything
        self.child.paint(ctx)
    }
}

impl WidgetState {
    pub(crate) fn should_receive_mouse(&self) -> bool {
        self.mouse_focus || self.hovered || self.child_mouse_focus
    }

    fn merge_up(&mut self, child: &mut WidgetState) {
        self.child_mouse_focus |= child.child_mouse_focus | child.mouse_focus;
        self.child_keyboard_focus |= child.child_keyboard_focus | child.keyboard_focus;
    }
}
