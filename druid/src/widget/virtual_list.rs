use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Range;
use std::sync::Arc;

use log::error;

use crate::widget::flex::Axis;
use crate::widget::{Label, ScrollControlState};
use crate::{
    BoxConstraints, BoxedWidget, Command, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, Point,
    Rect, RenderContext, Selector, Size, UpdateCtx, Widget, WidgetPod,
};
use std::collections::HashMap;

pub trait ListData<T, S>: Data
where
    T: Data + ToString + 'static,
    S: ScrollControlState,
{
    fn get_scroll_control_state(&self) -> &RefCell<S>;
    fn get_data(&self) -> Arc<Vec<T>>;
}
pub type RendererFunction<T> = Box<dyn FnMut(usize, Option<BoxedWidget<T>>) -> BoxedWidget<T>>;
pub type MeasuringFunction = Box<dyn FnMut(usize) -> f64>;
/// A list widget capable of millions or hundreds of millions
/// of items without degrading performance and provides enhanced
/// control over it's contents.
///
/// The VirtualList does not include a scrollbar widget. Instead,
/// it interacts with an implementor of the `ScrollControlState` trait
/// to get/set the scroll state. This allows developers the flexibility
/// of their own scroll mechanism or to wire in the `Scrollbar` widget
/// provided in this crate.
pub struct VirtualList<T, S, Ld>
where
    T: Data + ToString + 'static,
    S: ScrollControlState,
    Ld: ListData<T, S>,
{
    children: Vec<BoxedWidget<T>>,
    cached_children: Vec<BoxedWidget<T>>,
    data_range: Range<usize>,
    direction: Axis,
    /// Function used to measure list items. This
    /// should be cheap since all items are measured
    /// ahead of time when `variable_renderer_sizes' is true
    measuring_function: MeasuringFunction,
    scroll_delta: f64,
    scroll_position_cache: HashMap<usize, f64>,
    set_scroll_metrics_later: bool,
    /// Function to be called when a new widget
    /// is needed by the list. If a previously
    /// freed widget is available, it is provided
    /// and can optionally be used.
    renderer_function: RendererFunction<T>,
    renderer_size: f64,
    /// Flag indicating the items in the list
    /// have variable sizes and the `measuring_function`
    /// should be called to measure each one.
    /// Caution should be used since very large lists,
    /// or complex measuring algorithms can affect
    /// performance.
    variable_renderer_size: bool,

    list_data: PhantomData<Ld>,
    state: PhantomData<S>,
}

impl<T, S, Ld> VirtualList<T, S, Ld>
where
    T: Data + ToString + 'static,
    S: ScrollControlState,
    Ld: ListData<T, S>,
{
    pub fn new() -> VirtualList<T, S, Ld> {
        VirtualList {
            children: Vec::new(),
            cached_children: Vec::new(),
            data_range: 0..0,
            direction: Axis::Vertical,
            measuring_function: Box::new(|_| -> f64 { 30. }),
            scroll_delta: 0.,
            scroll_position_cache: HashMap::new(),
            renderer_function: Box::new(
                |_idx: usize, freed_widget: Option<BoxedWidget<T>>| -> BoxedWidget<T> {
                    freed_widget.unwrap_or_else(|| {
                        WidgetPod::new(Box::new(Label::new(|d: &T, _env: &Env| d.to_string())))
                    })
                },
            ),
            renderer_size: 30.,
            set_scroll_metrics_later: false,
            variable_renderer_size: false,

            list_data: Default::default(),
            state: Default::default(),
        }
    }

    pub fn direction(mut self, val: Axis) -> Self {
        self.direction = val;
        self
    }

    pub fn measuring_function(mut self, val: MeasuringFunction) -> Self {
        self.measuring_function = val;
        self
    }

    pub fn renderer_function(mut self, val: RendererFunction<T>) -> Self {
        self.renderer_function = val;
        self
    }

    pub fn variable_renderer_size(mut self, val: bool) -> Self {
        self.variable_renderer_size = val;
        self
    }

    pub fn renderer_size(mut self, val: f64) -> Self {
        self.renderer_size = val;
        self
    }

    fn get_preferred_renderer_size(&mut self, index: usize) -> f64 {
        if !self.variable_renderer_size {
            return self.renderer_size;
        }
        (self.measuring_function)(index)
    }

    fn get_index_at_scroll_position(&mut self, pos: f64, data: &Arc<Vec<T>>) -> usize {
        if pos <= 0. {
            return 0;
        }
        // Shortcut if we're not using variable renderer sizes
        if !self.variable_renderer_size {
            return (pos / self.renderer_size).ceil() as usize;
        }

        let len = data.len() - 1;
        let mut current_position = self.get_scroll_pos_at_index(len);
        // Estimate our target index
        let mut target_index = (pos / current_position) * len as f64;

        // Heuristic for reducing the number of
        // iterations needed to calculate the
        // index - .0075% iterations or less of
        // the list's length.
        while current_position > pos {
            target_index /= 1.0109375; // tested on indices of 5 or greater
            current_position = self.get_scroll_pos_at_index(target_index.ceil() as usize);
        }
        // At this point the current_position is
        // less than to the pos value but we're not sure
        // by how much.
        let mut last_pos = current_position;
        let target_position = current_position;

        for idx in target_index as usize..len {
            current_position =
                self.get_scroll_pos_at_index(idx) + self.get_preferred_renderer_size(idx);
            if current_position >= pos && last_pos <= pos {
                return idx;
            }
            last_pos = target_position;
        }
        0
    }

    fn get_scroll_pos_at_index(&mut self, index: usize) -> f64 {
        if !self.variable_renderer_size {
            return index as f64 * self.renderer_size;
        }
        if let Some(pos) = self.scroll_position_cache.get(&index) {
            return *pos;
        }
        // Nothing in cache - start with the last calculated index
        // and build the cache as we go.
        let start = *self.scroll_position_cache.keys().max().unwrap_or(&0);
        let mut pos = *self.scroll_position_cache.get(&start).unwrap_or(&0.);

        for idx in start..=index {
            self.scroll_position_cache.insert(idx, pos);
            pos += self.get_preferred_renderer_size(idx);
        }
        self.scroll_position_cache.get(&index).unwrap().clone()
    }

    /// Gets the start and end positions of the
    /// virtualized content depending on the Axis.
    fn get_content_metrics(&self) -> (f64, f64) {
        let len = self.children.len();
        if len == 0 {
            return (0., 0.);
        }
        let first = &self.children[0].get_layout_rect();
        let last = &self.children[len - 1].get_layout_rect();
        match self.direction {
            Axis::Vertical => (first.y0, last.y1),
            Axis::Horizontal => (first.x0, last.x1),
        }
    }

    /// Calculates the scroll_position, max_scroll_position
    /// and page_size based on the available width or height.
    fn set_scroll_metrics(&mut self, event_ctx: &mut EventCtx, list_data: &mut Ld) {
        let page_size = match self.direction {
            Axis::Vertical => event_ctx.size().height,
            Axis::Horizontal => event_ctx.size().width,
        };
        if page_size == 0. {
            self.set_scroll_metrics_later = true;
            event_ctx.request_anim_frame()
        }
        let mut scroll_control_state = list_data.get_scroll_control_state().borrow_mut();
        let data = list_data.get_data();
        let last_index = data.len() - 1;
        let new_max_scroll_position = self.get_scroll_pos_at_index(last_index);
        let new_max_size = self.get_preferred_renderer_size(last_index);

        scroll_control_state
            .set_max_scroll_position(new_max_scroll_position + new_max_size - page_size);
        scroll_control_state.set_page_size(page_size);
        // determine if we need to adjust the scroll_position.
        // This happens when a resize occurs on scrolled
        // content and no more rows can be displayed to fill
        // up the viewport.
        let (min, max) = self.get_content_metrics();
        if max < page_size && scroll_control_state.scroll_position() > 0. {
            scroll_control_state.set_scroll_pos_from_delta(-min);
        }
        event_ctx.invalidate();
    }

    /// Translates all children by the specified delta.
    /// Children outside the bounds are truncated.
    fn translate(&mut self, delta: f64, limit: f64) -> (f64, f64) {
        let (mut min, mut max) = self.get_content_metrics();
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
                let cm = match self.direction {
                    Axis::Vertical => (rect.y0, rect.y1),
                    Axis::Horizontal => (rect.x0, rect.x1),
                };

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
                    self.cached_children.push(self.children.remove(index));
                }
            }
            min -= delta;
            max -= delta;
        }

        (min, max)
    }
}

impl<T, S, Ld> Default for VirtualList<T, S, Ld>
where
    T: Data + ToString + 'static,
    S: ScrollControlState,
    Ld: ListData<T, S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S, Ld> Widget<Ld> for VirtualList<T, S, Ld>
where
    T: Data + ToString + 'static,
    S: ScrollControlState,
    Ld: ListData<T, S>,
{
    fn event(&mut self, event_ctx: &mut EventCtx, event: &Event, list_data: &mut Ld, _env: &Env) {
        match event {
            Event::Wheel(event) => {
                let mut scroll_control_state = list_data.get_scroll_control_state().borrow_mut();
                if !scroll_control_state.mouse_wheel_enabled() {
                    return;
                }
                let delta = match self.direction {
                    Axis::Vertical => event.delta.y,
                    Axis::Horizontal => event.delta.x,
                };
                scroll_control_state.set_scroll_pos_from_delta(delta);
                event_ctx.invalidate();

                let selector = Selector::new("scroll");
                let command = Command::new(selector, scroll_control_state.id());
                event_ctx.submit_command(command, None);
            }

            Event::MouseMoved(event) => {
                let mut scroll_control_state = list_data.get_scroll_control_state().borrow_mut();
                if !scroll_control_state.tracking_mouse() {
                    return;
                }
                let pos = match self.direction {
                    Axis::Vertical => event.pos.y,
                    Axis::Horizontal => event.pos.x,
                };

                let delta = pos - scroll_control_state.last_mouse_pos();
                let scale = scroll_control_state.scale();
                scroll_control_state.set_scroll_pos_from_delta(delta / scale);
                scroll_control_state.set_last_mouse_pos(pos);
                event_ctx.invalidate();
            }

            Event::MouseUp(_) => {
                let mut scroll_control_state = list_data.get_scroll_control_state().borrow_mut();
                scroll_control_state.set_tracking_mouse(false);
            }

            Event::Size(_) => {
                self.set_scroll_metrics(event_ctx, list_data);
            }

            Event::AnimFrame(_) => {
                if self.set_scroll_metrics_later {
                    self.set_scroll_metrics_later = false;
                    self.set_scroll_metrics(event_ctx, list_data);
                }
            }

            _ => (),
        }
    }

    fn update(
        &mut self,
        update_ctx: &mut UpdateCtx,
        old_data: Option<&Ld>,
        list_data: &Ld,
        _env: &Env,
    ) {
        if let Some(old_data) = old_data {
            let old_scroll_position = old_data
                .get_scroll_control_state()
                .borrow()
                .scroll_position();
            let new_scroll_position = list_data
                .get_scroll_control_state()
                .borrow()
                .scroll_position();
            let delta = new_scroll_position - old_scroll_position;
            if delta != 0. {
                self.scroll_delta += delta;
                update_ctx.invalidate();
            }
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        list_data: &Ld,
        _env: &Env,
    ) -> Size {
        let bounds = match self.direction {
            Axis::Vertical => bc.max().height,
            Axis::Horizontal => bc.max().width,
        };
        let (mut min, mut max) = self.translate(self.scroll_delta, bounds);
        // We've translated more than the viewport distance
        // and need to jump to a new data_range
        let scroll_control_state = list_data.get_scroll_control_state().borrow();
        let data = list_data.get_data();
        if self.children.is_empty() {
            let scroll_position = scroll_control_state.scroll_position();
            let index = self.get_index_at_scroll_position(scroll_position, &list_data.get_data());
            self.data_range = index..index;

            max = self.get_scroll_pos_at_index(index) - scroll_position;
            min = max;
        }
        // List items must attempt to fill the given box constraints.
        // Determine if we need to add items behind the start index (scroll_position increasing)
        while self.data_range.start != 0 && min > 0. {
            let index = self.data_range.start - 1;
            let widget = (self.renderer_function)(index, self.cached_children.pop());
            self.data_range.start -= 1;
            self.children.insert(0, widget);

            min -= self.get_preferred_renderer_size(index);
        }

        // determine if we need to add items in front of the end index
        while max < bounds && self.data_range.end < data.len() {
            let index = self.data_range.end;
            let widget = (self.renderer_function)(index, self.cached_children.pop());
            self.children.push(widget);
            self.data_range.end += 1;

            max += self.get_preferred_renderer_size(index);
        }

        // Layout all children starting from `min`
        let bc_max = bc.max();
        for index in self.data_range.start..self.data_range.end {
            let renderer_size = self.get_preferred_renderer_size(index);
            let (size, offset) = match self.direction {
                Axis::Horizontal => {
                    let size = Size::new(renderer_size, bc_max.height);
                    let offset = Point::new(min, 0.);
                    (size, offset)
                }
                Axis::Vertical => {
                    let size = Size::new(bc_max.width, renderer_size);
                    let offset = Point::new(0., min);
                    (size, offset)
                }
            };
            let rect = Rect::from_origin_size(offset, size);
            self.children[index - self.data_range.start].set_layout_rect(rect);

            min += renderer_size;
        }

        self.scroll_delta = 0.;
        bc.max()
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, list_data: &Ld, env: &Env) {
        if let Err(e) = paint_ctx.save() {
            error!("saving render context failed: {:?}", e);
            return;
        }
        let viewport = Rect::from_origin_size(Point::ORIGIN, paint_ctx.size());
        paint_ctx.clip(viewport);

        for (index, child) in &mut self.children.iter_mut().enumerate() {
            let idx = self.data_range.start + index;
            child.paint_with_offset(paint_ctx, &list_data.get_data()[idx], env);
        }

        if let Err(e) = paint_ctx.restore() {
            error!("restoring render context failed: {:?}", e);
        }
    }
}
