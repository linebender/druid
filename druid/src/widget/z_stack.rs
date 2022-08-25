use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, Size, UnitPoint, UpdateCtx, Vec2, Widget, WidgetExt, WidgetPod,
};

/// A container that stacks its children on top of each other.
///
/// The container has a baselayer which has the lowest z-index and determines the size of the
/// container.
pub struct ZStack<T> {
    layers: Vec<ZChild<T>>,
}

struct ZChild<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    relative_size: Vec2,
    absolute_size: Vec2,
    position: UnitPoint,
    offset: Vec2,
}

impl<T: Data> ZStack<T> {
    /// Creates a new ZStack with a baselayer.
    ///
    /// The baselayer is used by the ZStack to determine its own size.
    pub fn new(base_layer: impl Widget<T> + 'static) -> Self {
        Self {
            layers: vec![ZChild {
                child: WidgetPod::new(base_layer.boxed()),

                relative_size: Vec2::new(1.0, 1.0),
                absolute_size: Vec2::ZERO,
                position: UnitPoint::CENTER,
                offset: Vec2::ZERO,
            }],
        }
    }

    /// Builder-style method to add a new child to the Z-Stack.
    ///
    /// The child is added directly above the base layer.
    ///
    /// `relative_size` is the space the child is allowed to take up relative to its parent. The
    ///                 values are between 0 and 1.
    /// `absolute_size` is a fixed amount of pixels added to `relative_size`.
    ///
    /// `position`      is the alignment of the child inside the remaining space of its parent.
    ///
    /// `offset`        is a fixed amount of pixels added to `position`.
    pub fn with_child(
        mut self,
        child: impl Widget<T> + 'static,
        relative_size: Vec2,
        absolute_size: Vec2,
        position: UnitPoint,
        offset: Vec2,
    ) -> Self {
        let next_index = self.layers.len() - 1;
        self.layers.insert(
            next_index,
            ZChild {
                child: WidgetPod::new(child.boxed()),
                relative_size,
                absolute_size,
                position,
                offset,
            },
        );
        self
    }

    /// Builder-style method to add a new child to the Z-Stack.
    ///
    /// The child is added directly above the base layer, is positioned in the center and has no
    /// size constrains.
    pub fn with_centered_child(self, child: impl Widget<T> + 'static) -> Self {
        self.with_aligned_child(child, UnitPoint::CENTER)
    }

    /// Builder-style method to add a new child to the Z-Stack.
    ///
    /// The child is added directly above the base layer, uses the given alignment and has no
    /// size constrains.
    pub fn with_aligned_child(self, child: impl Widget<T> + 'static, alignment: UnitPoint) -> Self {
        self.with_child(
            child,
            Vec2::new(1.0, 1.0),
            Vec2::ZERO,
            alignment,
            Vec2::ZERO,
        )
    }
}

impl<T: Data> Widget<T> for ZStack<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let is_pointer_event = matches!(
            event,
            Event::MouseDown(_) | Event::MouseMove(_) | Event::MouseUp(_) | Event::Wheel(_)
        );

        for layer in self.layers.iter_mut() {
            layer.child.event(ctx, event, data, env);

            if is_pointer_event && layer.child.is_hot() {
                ctx.set_handled();
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        let mut previous_hot = false;
        for layer in self.layers.iter_mut() {
            let inner_event = event.ignore_hot(previous_hot);
            layer.child.lifecycle(ctx, &inner_event, data, env);
            previous_hot |= layer.child.is_hot();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, data: &T, env: &Env) {
        for layer in self.layers.iter_mut().rev() {
            layer.child.update(ctx, data, env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        //Layout base layer

        let base_layer = self.layers.last_mut().unwrap();
        let base_size = base_layer.child.layout(ctx, bc, data, env);

        //Layout other layers
        let other_layers = self.layers.len() - 1;

        for layer in self.layers.iter_mut().take(other_layers) {
            let max_size = layer.resolve_max_size(base_size);
            layer
                .child
                .layout(ctx, &BoxConstraints::new(Size::ZERO, max_size), data, env);
        }

        //Set origin for all Layers and calculate paint insets
        let mut previous_child_hot = false;
        let mut paint_rect = Rect::ZERO;

        for layer in self.layers.iter_mut() {
            let remaining = base_size - layer.child.layout_rect().size();
            let origin = layer.resolve_point(remaining);
            layer.child.set_origin(ctx, data, env, origin);

            paint_rect = paint_rect.union(layer.child.paint_rect());
            previous_child_hot |= layer.child.is_hot();
        }

        ctx.set_paint_insets(paint_rect - base_size.to_rect());
        ctx.set_baseline_offset(self.layers.last().unwrap().child.baseline_offset());

        base_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        //Painters algorithm (Painting back to front)
        for layer in self.layers.iter_mut().rev() {
            layer.child.paint(ctx, data, env);
        }
    }
}

impl<T: Data> ZChild<T> {
    fn resolve_max_size(&self, availible: Size) -> Size {
        self.absolute_size.to_size()
            + Size::new(
                availible.width * self.relative_size.x,
                availible.height * self.relative_size.y,
            )
    }

    fn resolve_point(&self, remaining_space: Size) -> Point {
        (self.position.resolve(remaining_space.to_rect()).to_vec2() + self.offset).to_point()
    }
}
