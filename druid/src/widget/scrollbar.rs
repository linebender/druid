use crate::{BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, RenderContext, UpdateCtx, Widget};
use crate::kurbo::{Rect, RoundedRect, Size};
use crate::theme;

/// Trait used for shared state between
/// the scrollbar and the scroll container.
pub trait ScrollControlState: Data + PartialEq {
    /// The last position reported by a MouseMove event.
    /// used to calculate the deltas without maintaining
    /// an offset.
    fn last_mouse_pos(&self) -> f64;
    /// The size of a page. Used to calculate
    /// scroll distances when clicking on the
    /// scroll track
    fn page_size(&self) -> f64;
    /// The maximum distance in pixels to scroll
    fn max_scroll_position(&self) -> f64;
    /// The minimum distance in pixels to scroll
    fn min_scroll_position(&self) -> f64;
    /// boolean for enabling/disabling the mouse
    fn mouse_wheel_enabled(&self) -> bool;
    /// scrollbar travel distance / scroll container travel distance
    fn scale(&self) -> f64;
    /// The current scroll position. This value
    /// is always between min and max scroll positions
    fn scroll_position(&self) -> f64;
    /// true when tracking mouse move events
    fn tracking_mouse(&self) -> bool;

    fn set_last_mouse_pos(&mut self, val: f64);
    fn set_page_size(&mut self, val: f64);
    fn set_max_scroll_position(&mut self, val: f64);
    fn set_min_scroll_position(&mut self, val: f64);
    fn set_mouse_wheel_enabled(&mut self, val: bool);
    fn set_tracking_mouse(&mut self, val: bool);
    fn set_scale(&mut self, val: f64);
    /// Sets the raw value for the scroll position
    /// without respecting min and max. Use this
    /// for pull or bounce calculations on the container
    fn set_scroll_position(&mut self, val: f64);

    fn set_scroll_pos_from_delta(&mut self, delta: f64) {
        let scroll_position = self.scroll_position() + delta;
        let min_scroll_position = self.min_scroll_position();
        let max_scroll_position = self.max_scroll_position();

        self.set_scroll_position(min_scroll_position.max(scroll_position.min(max_scroll_position)));
    }
}

pub struct Scrollbar {
    opacity: f64,
    scroll_policy: ScrollPolicy,
}

impl Scrollbar {
    pub fn new() -> Scrollbar {
        Scrollbar {
            opacity: 1.,
            scroll_policy: ScrollPolicy::Auto,
        }
    }

    pub fn scroll_policy(mut self, val: ScrollPolicy) -> Self {
        self.scroll_policy = val;
        self
    }

    fn calculated_thumb_size(data: &impl ScrollControlState, env: &Env, size: &Size) -> f64 {
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);
        let min_scroll_position = data.min_scroll_position();
        let max_scroll_position = data.max_scroll_position();
        let page_size = data.page_size();

        let extent = size.height.max(size.width);
        let target_thumb_size = (page_size / (max_scroll_position - min_scroll_position + page_size)) * extent;
        target_thumb_size.max(bar_width * 2.)
    }
}

impl<T: ScrollControlState> Widget<T> for Scrollbar {
    fn event(&mut self, event_ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let size = event_ctx.size();
        match event {
            Event::Wheel(event) => {
                if !data.mouse_wheel_enabled() {
                    return;
                }
                let delta = if size.width > size.height { event.delta.x } else { event.delta.y };
                data.set_scroll_pos_from_delta(delta);
                event_ctx.invalidate();
            }

            Event::MouseMoved(event) => {
                if !data.tracking_mouse() {
                    return;
                }
                let pos = if size.width > size.height { event.pos.x } else { event.pos.y };
                let delta = pos - data.last_mouse_pos();

                data.set_scroll_pos_from_delta(delta / data.scale());
                data.set_last_mouse_pos(pos);
                event_ctx.invalidate();
            }

            Event::MouseDown(event) => {
                data.set_tracking_mouse(true);
                data.set_last_mouse_pos(if size.width > size.height { event.pos.x } else { event.pos.y });
                // Set our scale since we could be dragging later
                // The thumb size is subtracted from the total
                // scrollable distance and a scale is calculated
                // to translate scrollbar distance to scroll container distance
                let distance = size.width.max(size.height);
                let thumb_size = Scrollbar::calculated_thumb_size(data, env, &size);
                data.set_scale((distance - thumb_size) / (data.max_scroll_position() - data.min_scroll_position()));
            }

            Event::MouseUp(_) => {
                data.set_tracking_mouse(false);
                data.set_last_mouse_pos(0.);
            }

            _ => ()
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: Option<&T>, data: &T, _env: &Env) {
        if let Some(old) = old_data {
            if old.max_scroll_position() != data.max_scroll_position()
                || old.min_scroll_position() != data.min_scroll_position()
                || old.scroll_position() != data.scroll_position() {

                ctx.invalidate();
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.constrain(bc.max())
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &T, env: &Env) {
        if data.max_scroll_position() < 0. {
            return;
        }
        let brush = paint_ctx.render_ctx.solid_brush(
            env.get(theme::SCROLL_BAR_COLOR)
                .with_alpha(self.opacity),
        );
        let border_brush = paint_ctx.render_ctx.solid_brush(
            env.get(theme::SCROLL_BAR_BORDER_COLOR)
                .with_alpha(self.opacity),
        );

        let radius = env.get(theme::SCROLL_BAR_RADIUS);
        let edge_width = env.get(theme::SCROLL_BAR_EDGE_WIDTH);
        let bar_width = env.get(theme::SCROLL_BAR_WIDTH);

        let size = paint_ctx.size();
        let min_scroll_position = data.min_scroll_position();
        let max_scroll_position = data.max_scroll_position();

        let extent = size.height.max(size.width);
        let thumb_size = Scrollbar::calculated_thumb_size(data, env, &size);
        let scale = (extent - thumb_size) / (max_scroll_position - min_scroll_position);
        let scaled_scroll_position = data.scroll_position() * scale;

        let bounds = if size.width > size.height {
            // Horizontal
            Rect::new(scaled_scroll_position, 0., scaled_scroll_position + thumb_size, bar_width * 2.)
        } else {
            // Vertical
            Rect::new(0., scaled_scroll_position, bar_width, scaled_scroll_position + thumb_size)
        };
        let rect = RoundedRect::from_rect(bounds, radius);
        paint_ctx.render_ctx.fill(rect, &brush);
        paint_ctx.render_ctx.stroke(rect, &border_brush, edge_width);
    }
}

pub enum ScrollPolicy {
    On,
    Off,
    Auto,
}