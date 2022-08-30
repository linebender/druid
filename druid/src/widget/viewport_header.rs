use druid::RenderContext;
use crate::{BoxConstraints, Color, Data, Env, Event, EventCtx, Insets, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, Size, UpdateCtx, ViewContext, Widget, WidgetExt, WidgetId, WidgetPod};
use crate::commands::SCROLL_TO_VIEW;
use crate::widget::flex::{Orientation, Side};


pub struct ViewportHeader<T> {
    header: WidgetPod<T, Box<dyn Widget<T>>>,
    content: WidgetPod<T, Box<dyn Widget<T>>>,

    header_config: ViewportHeaderConfig,
    clip_content: bool,
}

pub struct ViewportHeaderConfig {
    content_size: Size,
    viewport: Rect,
    header_side: Side,
    header_size: f64,
    minimum_visible_content: f64,
}

impl ViewportHeaderConfig {
    pub fn new(side: Side) -> Self {
        Self {
            content_size: Size::ZERO,
            viewport: Rect::from_origin_size(
                Point::ORIGIN,
                Size::new(f64::INFINITY, f64::INFINITY),
            ),
            header_side: side,
            header_size: 0.0,
            minimum_visible_content: 0.0,
        }
    }

    pub fn size(&self) -> Size {
        self.content_size + Size::from(self.header_side.axis().pack(self.header_size, 0.0))
    }

    pub fn side(&self) -> Side {
        self.header_side
    }

    /// The amount of pixels the header has be moved into the direction of content to stay inside
    /// of the viewport.
    ///
    /// The max value of this function is `major_content_size - minimum_visible_content`.
    /// Therefore header cant leave the content on the other side.
    pub fn overlapping(&self) -> f64 {
        //Compute Clipped area
        let global_layout_rect = Rect::from_origin_size(Point::ZERO, self.size());
        let insets = global_layout_rect - self.viewport;

        //Compute max movable distance
        let axis = self.header_side.axis();
        let max = axis.major(self.content_size) - self.minimum_visible_content;

        self.header_side.from_inset(insets).max(0.0).min(max)
    }

    /// The amount of pixels inside the viewport with is overlapped by the header.
    pub fn visual_overlapping(&self) -> f64 {
        self.overlapping().min(self.header_size)
    }

    /// Returns the origin of the content and of the header.
    pub fn origins(&self) -> (Point, Point) {
        let orientation = self.header_side.orientation();
        let axis = self.header_side.axis();

        let (first, _) = orientation.order(
            axis.major(self.content_size),
            self.header_size
        );

        let (content_origin, header_origin) = orientation.order(
            Point::ZERO,
            Point::from(axis.pack(first, 0.0))
        );
        let header_origin = header_origin - self.header_side.direction() * self.overlapping();

        (content_origin, header_origin)
    }

    pub fn transform_content_scroll_to_view(&self, ctx: &mut EventCtx, rect: Rect) {
        let axis = self.header_side.axis();
        // The length on the major axis with is overlapped by the header.
        let overlapping = self.visual_overlapping();

        if overlapping != 0.0 {
            ctx.set_handled();

            let new_rect = rect + self.header_side.direction() * overlapping;
            ctx.submit_notification_without_warning(SCROLL_TO_VIEW.with(new_rect));
        }
    }

    pub fn update_context(&mut self, view_context: ViewContext) {
        self.viewport = view_context.clip;
    }

    pub fn set_content_size(&mut self, content_size: Size) {
        self.content_size = content_size;
    }

    pub fn set_header_size(&mut self, header_size: Size) {
        let axis = self.header_side.axis();
        self.header_size = axis.major(header_size);
    }

    pub fn set_minimum_visible_content(&mut self, visible: f64) {
        self.minimum_visible_content = visible;
    }

}

impl<T: Data> ViewportHeader<T> {
    pub fn new(content: impl Widget<T> + 'static, header: impl Widget<T> + 'static, side: Side) -> Self {
        Self {
            header: WidgetPod::new(Box::new(header)),
            content: WidgetPod::new(Box::new(content)),
            header_config: ViewportHeaderConfig::new(side),
            clip_content: false,
        }
    }

    pub fn with_minimum_visible_content(mut self, minimum_visible_content: f64) -> Self {
        self.header_config.set_minimum_visible_content(minimum_visible_content);
        self
    }

    pub fn clipped_content(mut self, clipped_content: bool) -> Self {
        self.clip_content = clipped_content;
        self
    }
}

impl<T: Data> Widget<T> for ViewportHeader<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if let Event::Notification(notification) = event {
            if let Some(rect) = notification.get(SCROLL_TO_VIEW) {
                if notification.route() == self.content.id() {
                    self.header_config.transform_content_scroll_to_view(ctx, *rect);
                }
                return;
            }
        }

        self.header.event(ctx, event, data, env);
        if self.header.is_hot() && event.is_pointer_event() {
            ctx.set_handled();
        }
        self.content.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        match event {
            LifeCycle::ViewContextChanged(view_context) => {
                println!("update ctx");
                self.header_config.update_context(*view_context);
                let (_, header_origin) = self.header_config.origins();

                self.header.set_origin(ctx, data, env, header_origin);
                self.header.lifecycle(ctx, event, data, env);

                let mut content_view_context = *view_context;
                if self.header.is_hot() {
                    content_view_context.last_mouse_position = None;
                }
                content_view_context.clip = content_view_context.clip -
                    self.header_config.side().as_insets(self.header_config.visual_overlapping());

                self.content.lifecycle(ctx, event, data, env);
            }
            LifeCycle::BuildFocusChain if self.header_config.side().orientation() == Orientation::End => {
                self.content.lifecycle(ctx, event, data, env);
                self.header.lifecycle(ctx, event, data, env);
            }
            _ => {
                self.header.lifecycle(ctx, event, data, env);
                self.content.lifecycle(ctx, event, data, env);
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.header.update(ctx, data, env);
        self.content.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let axis = self.header_config.side().axis();

        let content_size = self.content.layout(ctx, bc, data, env);
        self.header_config.set_content_size(content_size);
        let header_bc = BoxConstraints::new(
            Size::from(axis.pack(0.0, axis.minor(content_size))),
            Size::from(axis.pack(f64::INFINITY, axis.minor(content_size))),
        );

        let header_size = self.header.layout(ctx, &header_bc, data, env);
        self.header_config.set_header_size(header_size);

        let (content_origin, header_origin) = self.header_config.origins();

        self.header.set_origin(ctx, data, env, header_origin);
        self.content.set_origin(ctx, data, env, content_origin);

        self.header_config.size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_save(|ctx| {
            if self.clip_content {
                let content_rect = self.content.layout_rect() -
                    self.header_config.side().as_insets(self.header_config.overlapping());
                ctx.clip(content_rect);
            }
            self.content.paint(ctx, data, env);
        });
        self.header.paint(ctx, data, env);
    }
}