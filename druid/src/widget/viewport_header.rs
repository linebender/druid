use crate::commands::SCROLL_TO_VIEW;
use crate::widget::flex::{Orientation, Side};
use crate::{
    BoxConstraints, Color, Data, Env, Event, EventCtx, Insets, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Point, Rect, Size, UpdateCtx, ViewContext, Widget, WidgetExt, WidgetId, WidgetPod,
};
use druid::RenderContext;

/// A widget, containing two widgets with horizontal or vertical layout.
///
/// When the `ViewportHeader` is moved out of the viewport, the `header` widget tries to stay inside
/// the viewport by moving over the `content` if necessary.
pub struct ViewportHeader<T> {
    header: WidgetPod<T, Box<dyn Widget<T>>>,
    content: WidgetPod<T, Box<dyn Widget<T>>>,

    header_config: ViewportHeaderConfig,
    clip_content: bool,
}

/// ViewportHeaderConfig contains the information necessary to create the layout of [`ViewportHeader`]
pub struct ViewportHeaderConfig {
    content_size: Size,
    viewport: Rect,
    header_side: Side,
    header_size: f64,
    minimum_visible_content: f64,
}

impl ViewportHeaderConfig {
    /// creates a new config.
    ///
    /// side: the side at which the header is located.
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

    /// The the layout size of header and content together, when both are fully in view.
    pub fn size(&self) -> Size {
        self.content_size + Size::from(self.header_side.axis().pack(self.header_size, 0.0))
    }

    /// The side of the header.
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

    /// The amount of pixels the viewport of the content gets cropped by the header.
    pub fn viewport_crop(&self) -> f64 {
        self.overlapping().min(self.header_size)
    }

    /// Returns the origin of the content and of the header.
    pub fn origins(&self) -> (Point, Point) {
        let orientation = self.header_side.orientation();
        let axis = self.header_side.axis();

        let (first, _) = orientation.order(axis.major(self.content_size), self.header_size);

        let (content_origin, header_origin) =
            orientation.order(Point::ZERO, Point::from(axis.pack(first, 0.0)));
        let header_origin = header_origin - self.header_side.direction() * self.overlapping();

        (content_origin, header_origin)
    }

    /// Updates a `scroll_to_view` request of the content to take the additional viewport crop into
    /// account.
    ///
    /// Dont call call this with requests of the header widget.
    pub fn transform_content_scroll_to_view(&self, ctx: &mut EventCtx, rect: Rect) {
        let axis = self.header_side.axis();
        // The length on the major axis with is overlapped by the header.
        let viewport_crop = self.viewport_crop();

        if viewport_crop != 0.0 {
            ctx.set_handled();

            let new_rect = rect + self.header_side.direction() * viewport_crop;
            ctx.submit_notification_without_warning(SCROLL_TO_VIEW.with(new_rect));
        }
    }

    /// Updates the ViewContext of the widget.
    ///
    /// Should be called when the widget receives a `Lifecycle::ViewContextChanged` event.
    pub fn update_context(&mut self, view_context: ViewContext) {
        self.viewport = view_context.clip;
    }

    /// Updates the content size.
    ///
    /// Should be called in layout.
    pub fn set_content_size(&mut self, content_size: Size) {
        self.content_size = content_size;
    }

    /// Updates the header size.
    ///
    /// should be called in layout
    pub fn set_header_size(&mut self, header_size: Size) {
        let axis = self.header_side.axis();
        self.header_size = axis.major(header_size);
    }

    /// Sets the minimum visible content.
    pub fn set_minimum_visible_content(&mut self, visible: f64) {
        self.minimum_visible_content = visible;
    }
}

impl<T: Data> ViewportHeader<T> {
    /// Creates a new ViewportHeader widget with a given side for the header.
    pub fn new(
        content: impl Widget<T> + 'static,
        header: impl Widget<T> + 'static,
        side: Side,
    ) -> Self {
        Self {
            header: WidgetPod::new(Box::new(header)),
            content: WidgetPod::new(Box::new(content)),
            header_config: ViewportHeaderConfig::new(side),
            clip_content: false,
        }
    }

    /// The amount of Pixels
    pub fn with_minimum_visible_content(mut self, minimum_visible_content: f64) -> Self {
        self.header_config
            .set_minimum_visible_content(minimum_visible_content);
        self
    }

    /// Builder-style method to set whether the additional cropped viewport should be clipped from
    /// from the content.
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
                    // The content is additionally cropped by the header, therefore we move the scroll
                    // request by the amount
                    self.header_config
                        .transform_content_scroll_to_view(ctx, *rect);
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

                self.header.set_origin(ctx, header_origin);
                self.header.lifecycle(ctx, event, data, env);

                let mut content_view_context = *view_context;
                if self.header.is_hot() {
                    content_view_context.last_mouse_position = None;
                }
                content_view_context.clip = content_view_context.clip
                    - self
                        .header_config
                        .side()
                        .as_insets(self.header_config.viewport_crop());

                self.content.lifecycle(ctx, event, data, env);
            }
            LifeCycle::BuildFocusChain
                if self.header_config.side().orientation() == Orientation::End =>
            {
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

        self.header.set_origin(ctx, header_origin);
        self.content.set_origin(ctx, content_origin);

        self.header_config.size()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_save(|ctx| {
            if self.clip_content {
                let content_rect = self.content.layout_rect()
                    - self
                        .header_config
                        .side()
                        .as_insets(self.header_config.overlapping());
                ctx.clip(content_rect);
            }
            self.content.paint(ctx, data, env);
        });
        self.header.paint(ctx, data, env);
    }
}
