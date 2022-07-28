use crate::{Data, Point, Size, Widget, WidgetPod, WidgetExt, EventCtx, Event, Env, LifeCycleCtx, LifeCycle, UpdateCtx, LayoutCtx, BoxConstraints, PaintCtx, Rect, Vec2};

pub struct ZStack<T> {
    layers: Vec<ZChild<T>>,
    base_layer: usize,
    current_hot: Option<usize>,
}

struct ZChild<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    size: LinearVec2,
    position: LinearVec2,
}

pub struct LinearVec2 {
    pub relative: Vec2,
    pub absolute: Vec2,
}

impl<T: Data> ZStack<T> {
    pub fn new(base_layer: impl Widget<T> + 'static) -> Self {
        Self {
            layers: vec![ZChild{
                child: WidgetPod::new(base_layer.boxed()),
                size: LinearVec2::full(),
                position: LinearVec2::empty(),
            }],
            base_layer: 0,
            current_hot: None,
        }
    }

    pub fn with_child_at_index(mut self, child: impl Widget<T> + 'static, position: LinearVec2, size: LinearVec2, index: usize) -> Self {
        self.layers.insert(index, ZChild {
            child: WidgetPod::new(child.boxed()),
            position,
            size,
        });
        if index < self.base_layer {
            // Baselayer moves down
            self.base_layer += 1;
        }
        self
    }
}

impl<T: Data> Widget<T> for ZStack<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let is_pointer_event = matches!(event, Event::MouseDown(_) | Event::MouseMove(_) | Event::MouseUp(_) | Event::Wheel(_));
        let mut previous_child_hot = false;

        self.current_hot = None;

        for (index, layer) in self.layers.iter_mut().enumerate() {
            if is_pointer_event && previous_child_hot {
                layer.child.event(ctx, &event.set_obstructed(), data, env);
            } else {
                layer.child.event(ctx, event, data, env);
            }
            if layer.child.is_hot() {
                self.current_hot = Some(index);
                previous_child_hot = true;
            }
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
        let base_layer = &mut self.layers[self.base_layer];
        let mut inner_ctx = LayoutCtx {
            state: ctx.state,
            widget_state: ctx.widget_state,
            mouse_pos: if Some(self.base_layer) == self.current_hot { ctx.mouse_pos } else { None },
        };

        let base_size = base_layer.child.layout(&mut inner_ctx, bc, data, env);
        base_layer.child.set_origin(&mut inner_ctx, data, env, Point::ORIGIN);
        ctx.set_baseline_offset(base_layer.child.baseline_offset());

        for (index, layer) in self.layers.iter_mut().enumerate() {
            let mut inner_ctx = LayoutCtx {
                state: ctx.state,
                widget_state: ctx.widget_state,
                mouse_pos: if Some(index) == self.current_hot { ctx.mouse_pos } else { None },
            };

            let max_size = layer.size.resolve(base_size.to_vec2());
            let size = layer.child.layout(&mut inner_ctx, &BoxConstraints::new(Size::ZERO, max_size.to_size()), data, env);
            let remaining = (base_size - size).to_vec2();
            let origin = layer.position.resolve(remaining);
            layer.child.set_origin(&mut inner_ctx, data, env, origin.to_point());
        }

        base_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        for layer in self.layers.iter_mut().rev() {
            layer.child.paint(ctx, data, env);
        }
    }
}

impl LinearVec2 {
    pub fn new(relative: impl Into<Vec2>, absolute: impl Into<Vec2>) -> Self {
        Self {
            relative: relative.into(),
            absolute: absolute.into(),
        }
    }

    pub fn full() -> Self {
        Self {
            relative: Vec2::new(1.0, 1.0),
            absolute: Vec2::new(0.0, 0.0),
        }
    }

    pub fn empty() -> Self {
        Self {
            relative: Vec2::new(0.0, 0.0),
            absolute: Vec2::new(0.0, 0.0),
        }
    }

    pub fn from_absolute(absolute: impl Into<Vec2>) -> Self {
        Self::new(Vec2::ZERO, absolute)
    }

    pub fn from_relative(relative: impl Into<Vec2>) -> Self {
        Self::new(relative, Vec2::ZERO)
    }

    pub fn resolve(&self, reference: Vec2) -> Vec2 {
        Vec2::new(
            self.absolute.x + self.relative.x * reference.x,
            self.absolute.y + self.relative.y * reference.y
        )
    }

    pub fn resolve_external(&self, reference: Vec2) -> Vec2 {
        Vec2::new(
            (reference.x + self.absolute.x) * self.relative.x / (1.0 - self.relative.x),
            (reference.y + self.absolute.y) * self.relative.y / (1.0 - self.relative.y),
        )
    }
}