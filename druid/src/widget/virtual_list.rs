use std::marker::PhantomData;
use std::ops::Range;

use log::error;

use crate::{BoxConstraints, BoxedWidget, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Widget, WidgetPod, Selector, Command};
use crate::widget::{Label, ScrollControlState, WidgetExt};
use crate::widget::flex::Axis;

pub struct VirtualList<T: Data + ToString, S: ScrollControlState> {
    children: Vec<BoxedWidget<T>>,
    // The rect representing the area occupied by
    // all visible widgets. This will always be
    // equal to or larger than the BoxConstraints
    // when virtualized.
    content_metrics: Rect,
    data_range: Range<usize>,
    data_provider: Vec<T>,
    direction: Axis,
    scroll_delta: f64,
    // Always represents the topmost/rightmost position depending on the direction
    renderer_function: fn(data: &T) -> Box<dyn Widget<T>>,
    renderer_size: f64,
    set_scroll_metrics_later: bool,

    phantom_data: PhantomData<S>,
}

impl<T: Data + ToString + 'static, S: ScrollControlState> VirtualList<T, S> {
    pub fn new() -> VirtualList<T, S> {
        VirtualList {
            children: Vec::new(),
            content_metrics: Rect::new(0., 0., 0., 0.),
            data_range: 0..0,
            data_provider: Vec::new(),
            direction: Axis::Vertical,
            scroll_delta: 0.,
            renderer_function: |data: &T| -> Box<dyn Widget<T>> { Box::new(Label::new(data.to_string()).fix_height(30.)) },
            renderer_size: 30.,
            set_scroll_metrics_later: false,
            phantom_data: Default::default(),
        }
    }

    pub fn renderer_function(mut self, val: fn(data: &T) -> Box<dyn Widget<T>>) -> Self {
        self.renderer_function = val;
        self
    }

    pub fn direction(mut self, val: Axis) -> Self {
        self.direction = val;
        self
    }

    pub fn data_provider(mut self, val: Vec<T>) -> Self {
        self.data_provider = val;
        self
    }

    pub fn renderer_size(mut self, val: f64) -> Self {
        self.renderer_size = val;
        self
    }

    fn set_content_metrics(&mut self, value: (f64, f64)) {
        match self.direction {
            Axis::Vertical => {
                self.content_metrics.y0 = value.0;
                self.content_metrics.y1 = value.1;
            }
            Axis::Horizontal => {
                self.content_metrics.x0 = value.0;
                self.content_metrics.x1 = value.1;
            }
        };
    }

    fn set_scroll_metrics(&mut self, event_ctx: &mut EventCtx, data: &mut S) {
        // Recalculate the max_scroll_position
        let page_size = match self.direction {
            Axis::Vertical => event_ctx.size().height,
            Axis::Horizontal => event_ctx.size().width
        };
        if page_size == 0. {
            self.set_scroll_metrics_later = true;
            event_ctx.request_anim_frame()
        }
        data.set_max_scroll_position((self.data_provider.len() as f64 * self.renderer_size) - page_size);
        data.set_page_size(page_size);
        event_ctx.invalidate();
    }

    /// Translates all children by the specified delta.
    /// Children outside the 0..limit bounds are truncated
    fn translate(&mut self, delta: f64, limit: f64) -> (f64, f64) {
        let (mut min, mut max) = get_scroll_metrics(&self.direction, &self.content_metrics);
        if delta != 0. {
            // TODO - replace implementation with Vec::drain_filter once it's stable.
            let mut to_remove = Vec::new();
            for (index, child) in &mut self.children.iter_mut().enumerate() {
                let mut rect = child.get_layout_rect();
                match self.direction {
                    Axis::Vertical => {
                        rect = rect.with_origin(Point::new(0., rect.y0 - delta));
                    }
                    Axis::Horizontal => {
                        rect = rect.with_origin(Point::new(rect.x0 - delta, 0.));
                    }
                }
                let cm = get_scroll_metrics(&self.direction, &rect);

                if cm.1 < 0. {
                    // Child is less than the viewport's min
                    to_remove.push(index);
                    min += cm.1 - cm.0;
                    self.data_range.start += 1;
                } else if cm.0 > limit {
                    // Child is greater than the viewport's max
                    to_remove.push(index);
                    max -= cm.1 - cm.0;
                    self.data_range.end -= 1;
                } else {
                    child.set_layout_rect(rect);
                }
            }
            // Truncate children if necessary
            if !to_remove.is_empty() {
                to_remove.sort_by(|a, b| b.cmp(a));
                for index in to_remove {
                    self.children.remove(index);
                }
            }
            min -= delta;
            max -= delta;
        }

        (min, max)
    }
}

impl<T: Data + ToString +  'static, S: ScrollControlState> Default for VirtualList<T, S> {
    fn default() -> Self {
        VirtualList {
            children: Vec::new(),
            content_metrics: Default::default(),
            data_range: 0..0,
            data_provider: Vec::new(),
            direction: Axis::Vertical,
            scroll_delta: 0.,
            renderer_function: |data: &T| -> Box<dyn Widget<T>> { Box::new(Label::new(data.to_string()).fix_height(30.)) },
            renderer_size: 0.,
            set_scroll_metrics_later: false,
            phantom_data: PhantomData
        }
    }
}

impl<T: Data + ToString + 'static, S: ScrollControlState, > Widget<S> for VirtualList<T, S> {
    fn event(&mut self, event_ctx: &mut EventCtx, event: &Event, data: &mut S, _env: &Env) {
        match event {
            Event::Wheel(event) => {
                if !data.mouse_wheel_enabled() {
                    return;
                }
                let delta = match self.direction {
                    Axis::Vertical => event.delta.y,
                    Axis::Horizontal => event.delta.x
                };
                data.set_scroll_pos_from_delta(delta);
                event_ctx.invalidate();

                let selector = Selector::new("scroll");
                let command = Command::new(selector, data.id());
                event_ctx.submit_command(command, None)
            }

            Event::MouseMoved(event) => {
                if !data.tracking_mouse() {
                    return;
                }
                let pos = match self.direction {
                    Axis::Vertical => event.pos.y,
                    Axis::Horizontal => event.pos.x
                };

                let delta = pos - data.last_mouse_pos();

                data.set_scroll_pos_from_delta(delta / data.scale());
                data.set_last_mouse_pos(pos);
                event_ctx.invalidate();
            }

            Event::MouseUp(_) => {
                data.set_tracking_mouse(false);
            }

            Event::Size(_) => {
                self.set_scroll_metrics(event_ctx, data);
            },

            Event::AnimFrame(_) => {
                if self.set_scroll_metrics_later {
                    self.set_scroll_metrics_later = false;
                    self.set_scroll_metrics(event_ctx, data);
                }
            }

            _ => ()
        }
    }

    fn update(&mut self, update_ctx: &mut UpdateCtx, old_data: Option<&S>, data: &S, _env: &Env) {
        if let Some(old_data) = old_data {
            let old_scroll_position = old_data.scroll_position();
            let new_scroll_position = data.scroll_position();
            let delta = new_scroll_position - old_scroll_position;
            if delta != 0. {
                self.scroll_delta += delta;
                update_ctx.invalidate();
            }
        }
    }

    fn layout(&mut self, layout_ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &S, env: &Env) -> Size {
        let bounds = match self.direction {
            Axis::Vertical => bc.max().height,
            Axis::Horizontal => bc.max().width
        };
        let (mut min, mut max) = self.translate(self.scroll_delta, bounds);
        // We've translated more than the viewport distance
        // and need to jump to a new data_range
        if self.children.is_empty() {
            let fractional_index = data.scroll_position() / self.renderer_size;
            let index = fractional_index.floor() as usize;
            self.data_range = index..index;
            min = 0.;
            max = (index as f64 * self.renderer_size) - (fractional_index * self.renderer_size);
        }
        // List items must attempt to fill the given box constraints.
        // Determine if we need to add items behind the start index (scroll_position increasing)
        while self.data_range.start != 0 && min > 0. {
            if let Some(data) = self.data_provider.get(self.data_range.start - 1) {
                let mut widget = WidgetPod::new((self.renderer_function)(data));
                let child_bc = BoxConstraints::new(Size::ZERO, bc.max());
                let child_size = widget.layout(layout_ctx, &child_bc, data, env);

                let mut offset = Point::new(0., 0.);
                min -= match self.direction {
                    Axis::Horizontal => {
                        offset.x = min - child_size.width;
                        child_size.width
                    }
                    Axis::Vertical => {
                        offset.y = min - child_size.height;
                        child_size.height
                    }
                };
                let rect = Rect::from_origin_size(offset, child_size);
                widget.set_layout_rect(rect);
                self.data_range.start -= 1;
                self.children.insert(0, widget);
            } else {
                break;
            }
        }

        // determine if we need to add items in front of the end index

        while max < bounds {
            if let Some(data) = self.data_provider.get(self.data_range.end) {
                let mut widget = WidgetPod::new((self.renderer_function)(data));
                let child_bc = BoxConstraints::new(Size::ZERO, bc.max());
                let child_size = widget.layout(layout_ctx, &child_bc, data, env);
                let mut offset = Point::new(0., 0.);
                max += match self.direction {
                    Axis::Horizontal => {
                        offset.x = max;
                        child_size.width
                    }
                    Axis::Vertical => {
                        offset.y = max;
                        child_size.height
                    }
                };
                let rect = Rect::from_origin_size(offset, child_size);
                widget.set_layout_rect(rect);
                self.children.push(widget);
                self.data_range.end += 1;
            } else {
                break;
            }
        }

        self.set_content_metrics((min, max));
        self.scroll_delta = 0.;
        bc.max()
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, _data: &S, env: &Env) {
        if let Err(e) = paint_ctx.save() {
            error!("saving render context failed: {:?}", e);
            return;
        }
        let viewport = Rect::from_origin_size(Point::ORIGIN, paint_ctx.size());
        paint_ctx.clip(viewport);

        for (index, child) in &mut self.children.iter_mut().enumerate() {
            let idx = self.data_range.start + index;
            child.paint_with_offset(paint_ctx, &self.data_provider[idx], env);
        }

        if let Err(e) = paint_ctx.restore() {
            error!("restoring render context failed: {:?}", e);
        }
    }
}

fn get_scroll_metrics(direction: &Axis, rect: &Rect) -> (f64, f64) {
    match direction {
        Axis::Vertical => (rect.y0, rect.y1),
        Axis::Horizontal => (rect.x0, rect.x1)
    }
}

impl<T: Data> Data for Vec<T> {
    fn same(&self, other: &Self) -> bool {
        self.len() == other.len()
    }
}