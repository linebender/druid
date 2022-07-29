use crate::{Data, Size, Widget, WidgetPod, WidgetExt, EventCtx, Event, Env, LifeCycleCtx, LifeCycle, UpdateCtx, LayoutCtx, BoxConstraints, PaintCtx, Vec2, UnitPoint, Rect};

/// A container that stacks its children on top of each other.
///
/// The container has a baselayer which has the lowest z-index and determines the size of the
/// container.
pub struct ZStack<T> {
    layers: Vec<ZChild<T>>,
}

struct ZChild<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    size: LinearVec2,
    position: LinearVec2,
}

/// A two dimensional Vector relative to the available space.
pub struct LinearVec2 {
    pub relative: UnitPoint,
    pub absolute: Vec2,
}

impl<T: Data> ZStack<T> {
    /// Creates a new ZStack with a baselayer.
    ///
    /// The baselayer is used by the ZStack to determine its own size.
    pub fn new(base_layer: impl Widget<T> + 'static) -> Self {
        Self {
            layers: vec![ZChild{
                child: WidgetPod::new(base_layer.boxed()),
                size: LinearVec2::from_unit(UnitPoint::BOTTOM_RIGHT),
                position: LinearVec2::from_unit(UnitPoint::TOP_LEFT),
            }],
        }
    }

    /// Builder-style method to insert a new child into the Z-Stack.
    ///
    /// The index must be smaller that that of the base-layer.
    pub fn with_child_at_index(mut self, child: impl Widget<T> + 'static, position: LinearVec2, size: LinearVec2, index: usize) -> Self {
        assert!(index < self.layers.len());
        self.layers.insert(index, ZChild {
            child: WidgetPod::new(child.boxed()),
            position,
            size,
        });
        self
    }

    /// Builder-style method to add a new child to the Z-Stack.
    ///
    /// The child is added directly above the base layer.
    pub fn with_child(self, child: impl Widget<T> + 'static, position: LinearVec2, size: LinearVec2) -> Self {
        let next_index = self.layers.len() - 1;
        self.with_child_at_index(child, position, size, next_index)
    }
}

impl<T: Data> Widget<T> for ZStack<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let is_pointer_event = matches!(event, Event::MouseDown(_) | Event::MouseMove(_) | Event::MouseUp(_) | Event::Wheel(_));
        let mut previous_child_hot = false;

        for layer in self.layers.iter_mut() {
            layer.child.event(
                ctx,
                &event.set_obstructed(is_pointer_event && previous_child_hot),
                data,
                env
            );

            previous_child_hot |= layer.child.is_hot();
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        for layer in self.layers.iter_mut() {
            layer.child.lifecycle(ctx, event, data, env);
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
            let max_size = layer.size.resolve(base_size);
            layer.child.layout(
                ctx,
                &BoxConstraints::new(Size::ZERO, max_size.to_size()),
                data,
                env
            );
        }

        //Set origin for all Layers and calculate paint insets
        let mut previous_child_hot = false;
        let mut paint_rect = Rect::ZERO;

        for layer in self.layers.iter_mut() {
            let mut inner_ctx = ctx.set_obstructed(previous_child_hot);

            let remaining = base_size - layer.child.layout_rect().size();
            let origin = layer.position.resolve(remaining).to_point();
            layer.child.set_origin(&mut inner_ctx, data, env, origin);

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

impl LinearVec2 {
    /// Creates a new LinearVec2 from an UnitPoint and an offset.
    pub fn new(relative: impl Into<Vec2>, offset: impl Into<Vec2>) -> Self {
        Self {
            relative: relative.into(),
            absolute: offset.into(),
        }
    }

    /// creates a new LinearVec2 from a UnitPoint. Offset ist set to Zero.
    pub fn from_unit(relative: impl Into<UnitPoint>) -> Self {
        let point = relative.into();
        Self::new(Vec2::new(point.), Vec2::ZERO)
    }

    pub fn from_relative_size(relative: impl Into<Size>) -> Self {

    }

    /// resolves this LinearVec2 for a given size
    pub fn resolve(&self, reference: Size) -> Vec2 {
        self.relative.resolve(reference.to_rect()).to_vec2() + self.absolute
    }
}