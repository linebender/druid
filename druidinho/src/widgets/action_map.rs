use crate::kurbo::Size;
use crate::shell::{KeyEvent, TimerToken};
use crate::{BoxConstraints, EventCtx, LayoutCtx, MouseEvent, PaintCtx, Widget};

pub struct ActionMapper<W, In, Out> {
    child: W,
    //FIXME: this only has a context so that, during development, we can manually
    //trigget update as needed?
    map: Box<dyn FnMut(In, &mut EventCtx<Out>) -> Option<Out>>,
    actions: Vec<In>,
    temp_parent_messages: Vec<Out>,
}

impl<W, In, Out> ActionMapper<W, In, Out> {
    pub fn new(child: W, map: impl FnMut(In, &mut EventCtx<Out>) -> Option<Out> + 'static) -> Self {
        ActionMapper {
            child,
            map: Box::new(map),
            actions: Vec::new(),
            temp_parent_messages: Vec::new(),
        }
    }
}

impl<W, In, Out> ActionMapper<W, In, Out> {
    fn with_child<R>(
        &mut self,
        parent_ctx: &mut EventCtx<Out>,
        f: impl FnOnce(&mut W, &mut EventCtx<In>) -> R,
    ) -> R {
        let mut child_ctx = EventCtx {
            state: parent_ctx.state,
            window: parent_ctx.window,
            layout_state: parent_ctx.layout_state,
            messages: &mut self.actions,
            never_messages: Vec::new(),
        };
        let r = f(&mut self.child, &mut child_ctx);

        let ActionMapper { map, actions, .. } = self;

        std::mem::swap(&mut self.temp_parent_messages, parent_ctx.messages);
        let mapped = actions.drain(..).filter_map(|x| (map)(x, parent_ctx));
        self.temp_parent_messages.extend(mapped);
        self.temp_parent_messages
            .extend(parent_ctx.messages.drain(..));
        std::mem::swap(&mut self.temp_parent_messages, parent_ctx.messages);

        r
    }
}

impl<W: Widget<Action = In>, In, Out> Widget for ActionMapper<W, In, Out> {
    type Action = Out;

    fn init(&mut self, ctx: &mut EventCtx<Out>) {
        self.with_child(ctx, |chld, ctx| chld.init(ctx))
    }
    fn mouse_down(&mut self, ctx: &mut EventCtx<Out>, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.mouse_down(ctx, event));
    }
    fn mouse_up(&mut self, ctx: &mut EventCtx<Out>, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.mouse_up(ctx, event));
    }

    fn mouse_move(&mut self, ctx: &mut EventCtx<Out>, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.mouse_move(ctx, event));
    }
    fn scroll(&mut self, ctx: &mut EventCtx<Out>, event: &MouseEvent) {
        self.with_child(ctx, |chld, ctx| chld.scroll(ctx, event));
    }
    fn key_down(&mut self, ctx: &mut EventCtx<Out>, event: &KeyEvent) {
        self.with_child(ctx, |chld, ctx| chld.key_down(ctx, event));
    }

    fn key_up(&mut self, ctx: &mut EventCtx<Out>, event: &KeyEvent) {
        self.with_child(ctx, |chld, ctx| chld.key_up(ctx, event));
    }

    fn timer(&mut self, ctx: &mut EventCtx<Out>, token: TimerToken) {
        self.with_child(ctx, |chld, ctx| chld.timer(ctx, token));
    }

    fn update(&mut self) {
        self.child.update();
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: BoxConstraints) -> Size {
        self.child.layout(ctx, bc)
    }

    fn paint(&self, ctx: &mut PaintCtx) {
        self.child.paint(ctx)
    }
}
