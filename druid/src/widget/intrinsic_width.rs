use druid::Data;

use crate::{
    BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget,
};

/// A widget that sizes its child to the child's maximum intrinsic width.
///
/// This widget is useful, for example, when unlimited width is available and you would like a child
/// that would otherwise attempt to expand infinitely to instead size itself to a more reasonable
/// width.
///
/// The constraints that this widget passes to its child will adhere to the parent's
/// constraints, so if the constraints are not large enough to satisfy the child's maximum intrinsic
/// width, then the child will get less width than it otherwise would. Likewise, if the minimum
/// width constraint is larger than the child's maximum intrinsic width, the child will be given
/// more width than it otherwise would.
pub struct IntrinsicWidth<T> {
    child: Box<dyn Widget<T>>,
}

impl<T: Data> IntrinsicWidth<T> {
    /// Wrap the given `child` in this widget.
    pub fn new(child: impl Widget<T> + 'static) -> Self {
        Self {
            child: Box::new(child),
        }
    }
}

impl<T: Data> Widget<T> for IntrinsicWidth<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.child.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.child.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let mut bc = *bc;
        let iw = self.child.compute_max_intrinsic_width(ctx, &bc, data, env);
        bc.set_max_width(iw);

        self.child.layout(ctx, &bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.child.paint(ctx, data, env);
    }

    fn compute_max_intrinsic_width(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        self.child.compute_max_intrinsic_width(ctx, bc, data, env)
    }

    fn compute_max_intrinsic_height(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &T,
        env: &Env,
    ) -> f64 {
        if !bc.is_width_bounded() {
            let mut bc = *bc;
            let w = self.child.compute_max_intrinsic_width(ctx, &bc, data, env);
            bc.set_max_width(w);
            self.child.compute_max_intrinsic_height(ctx, &bc, data, env)
        } else {
            self.child.compute_max_intrinsic_height(ctx, bc, data, env)
        }
    }
}
