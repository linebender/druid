use std::ops::{Deref, DerefMut};

use crate::kurbo::{Point, Size};
use druid_shell::{KeyEvent, TimerToken};

use crate::contexts::{EventCtx, LayoutCtx, PaintCtx};
use crate::widgets::layout::LayoutHost;
use crate::{BoxConstraints, MouseEvent, Widget};

pub struct WidgetHost<W> {
    child: LayoutHost<W>,
    state: WidgetState,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WidgetState {
    ///// The mouse is inside the widget's frame.
    //pub(crate) hovered: bool,
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

impl WidgetState {
    pub(crate) fn has_mouse_focus(&self) -> bool {
        self.mouse_focus || self.child_mouse_focus
    }

    fn merge_up(&mut self, child: &mut WidgetState) {
        self.child_mouse_focus |= child.child_mouse_focus | child.mouse_focus;
        self.child_keyboard_focus |= child.child_keyboard_focus | child.keyboard_focus;
    }
}

impl<W: Widget> WidgetHost<W> {
    pub fn new(child: W) -> Self {
        WidgetHost {
            child: LayoutHost::new(child),
            state: Default::default(),
        }
    }

    pub fn set_origin(&mut self, origin: Point) {
        self.child.set_origin(origin);
    }

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
            layout_state: parent_ctx.layout_state,
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
        let mut child_ctx = LayoutCtx {
            layout_state: ctx.layout_state,
            state: &mut self.state,
            window: ctx.window,
        };
        // we always lay out eveything
        self.child.layout(&mut child_ctx, bc)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        let mut child_ctx = PaintCtx {
            layout_state: ctx.layout_state,
            state: &self.state,
            render_ctx: ctx.render_ctx,
        };
        // we always paint everything
        self.child.paint(&mut child_ctx)
    }
}

impl Widget for Box<dyn Widget> {
    fn init(&mut self, ctx: &mut EventCtx) {
        self.deref_mut().init(ctx)
    }
    fn mouse_down(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.deref_mut().mouse_down(ctx, event);
    }
    fn mouse_up(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.deref_mut().mouse_up(ctx, event);
    }

    fn mouse_move(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.deref_mut().mouse_move(ctx, event);
    }
    fn scroll(&mut self, ctx: &mut EventCtx, event: &MouseEvent) {
        self.deref_mut().scroll(ctx, event);
    }
    fn key_down(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        self.deref_mut().key_down(ctx, event);
    }

    fn key_up(&mut self, ctx: &mut EventCtx, event: &KeyEvent) {
        self.deref_mut().key_up(ctx, event);
    }

    fn timer(&mut self, ctx: &mut EventCtx, token: TimerToken) {
        self.deref_mut().timer(ctx, token);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        self.deref_mut().layout(ctx, bc)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        self.deref().paint(ctx)
    }
}
