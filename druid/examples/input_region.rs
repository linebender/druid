use druid::widget::{Widget, Flex, Label, Button, Container, LineBreaking};
use druid::widget::prelude::*;
use druid::{AppLauncher, Lens, Rect, WidgetPod, WidgetExt, Point, Color, Region, WindowDesc};

const EXAMPLE_BORDER_SIZE: f64 = 3.0;

#[derive(Clone, Data, Lens)]
struct AppState {
    limit_input_region: bool,
    show_titlebar: bool,
    always_on_top: bool,
}

struct InputRegionExampleWidget {
    info_label: WidgetPod<AppState, Container<AppState>>,
    controls: WidgetPod<AppState, Flex<AppState>>,
}

impl InputRegionExampleWidget {
    pub fn new() -> Self {
        let info_label = Label::new("Only this text and the borders can be interacted with. You can click through the other parts")
            .with_line_break_mode(LineBreaking::WordWrap)
            .padding(20.0)
            .background(Color::rgba(0.2, 0.2, 0.2, 1.0));
        let toggle_input_region = Button::new("Toggle Input Region")
            .on_click(|ctx, data: &mut bool, _: &Env| {
                *data = !*data;
                println!("Setting input region toggle to: {}", *data);
                ctx.request_layout();
            })
            .lens(AppState::limit_input_region);
        let toggle_titlebar = Button::new("Toggle TitleBar")
            .on_click(|ctx, data: &mut bool, _: &Env| {
                *data = !*data;
                println!("Setting titlebar visibility to: {}", *data);
                ctx.window().show_titlebar(*data);
                ctx.request_layout();
            })
            .lens(AppState::show_titlebar);
        let toggle_always_on_top = Button::new("Toggle Always On Top")
            .on_click(|ctx, data: &mut bool, _: &Env| {
                *data = !*data;
                println!("Setting always on top to: {}", *data);
                ctx.window().set_always_on_top(*data);
            })
            .lens(AppState::always_on_top);
        let controls_flex = Flex::row()
            .with_child(toggle_input_region)
            .with_child(toggle_titlebar)
            .with_child(toggle_always_on_top);
        Self {
            info_label: WidgetPod::new(info_label),
            controls: WidgetPod::new(controls_flex),
        }
    }
}

impl Widget<AppState> for InputRegionExampleWidget {
    fn event(&mut self, ctx: &mut druid::EventCtx, event: &druid::Event, data: &mut AppState, env: &druid::Env) {
        self.info_label.event(ctx, event, data, env);
        self.controls.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut druid::LifeCycleCtx, event: &druid::LifeCycle, data: &AppState, env: &druid::Env) {
        self.info_label.lifecycle(ctx, event, data, env);
        self.controls.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut druid::UpdateCtx, _old_data: &AppState, data: &AppState, env: &druid::Env) {
        self.info_label.update(ctx, data, env);
        self.controls.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut druid::LayoutCtx, bc: &druid::BoxConstraints, data: &AppState, env: &druid::Env) -> druid::Size {
        let mut interactable_area = Region::EMPTY;
        let half_size_bc = BoxConstraints::new(
            Size::new(0.0, 0.0),
            Size::new(bc.max().width / 2.0, bc.max().height / 2.0)
        );
        let full_bc = BoxConstraints::new(
            Size::new(0.0, 0.0),
            bc.max()
        );
        let _label_size = self.info_label.layout(ctx, &half_size_bc, data, env);
        let _controls_size = self.controls.layout(ctx, &full_bc, data, env);

        let text_origin_point = Point::new(bc.max().width / 4.0, bc.max().width / 4.0);
        self.info_label.set_origin(ctx, text_origin_point);
        let controls_origin_point = Point::new(EXAMPLE_BORDER_SIZE, EXAMPLE_BORDER_SIZE);
        self.controls.set_origin(ctx, controls_origin_point);

        // Add side rects to clarify the dimensions of the window.
        let left_rect = Rect::new(0.0, 0.0, EXAMPLE_BORDER_SIZE, bc.max().height);
        let right_rect = Rect::new(bc.max().width - EXAMPLE_BORDER_SIZE, 0.0, bc.max().width, bc.max().height);
        let bottom_rect = Rect::new(0.0, bc.max().height - EXAMPLE_BORDER_SIZE, bc.max().width, bc.max().height);
        interactable_area.add_rect(left_rect);
        interactable_area.add_rect(right_rect);
        interactable_area.add_rect(bottom_rect);
        interactable_area.add_rect(self.info_label.layout_rect());
        interactable_area.add_rect(self.controls.layout_rect());

        if data.limit_input_region {
            ctx.window().set_input_region(Some(interactable_area));
        } else {
            ctx.window().set_input_region(None);
        }

        bc.max()
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &AppState, env: &druid::Env) {
        let window_area = ctx.size();
        let left_rect = Rect::new(0.0, 0.0, EXAMPLE_BORDER_SIZE, window_area.height);
        let right_rect = Rect::new(window_area.width - 3.0, 0.0, window_area.width, window_area.height);
        let bottom_rect = Rect::new(0.0, window_area.height - EXAMPLE_BORDER_SIZE, window_area.width, window_area.height);

        ctx.fill(left_rect, &Color::rgba(1.0, 0., 0., 0.7));
        ctx.fill(right_rect, &Color::rgba(1.0, 0., 0., 0.7));
        ctx.fill(bottom_rect, &Color::rgba(1.0, 0., 0., 0.7));
        self.info_label.paint(ctx, data, env);
        self.controls.paint(ctx, data, env);
    }
}


fn main() {
    let main_window = WindowDesc::new(InputRegionExampleWidget::new())
        .title("Input Region Demo")
        .window_size((600.0, 300.0))
        // Disable the titlebar since it breaks the desired effect on mac.
        // It can be turned on with the button, but not on mac.
        // A lot of apps that will use the interaction features will turn this off
        // On Windows, if on, this will be invisible, but still there.
        .show_titlebar(false)
        .transparent(true);

    let state = AppState {
        limit_input_region: true,
        always_on_top: false,
        show_titlebar: false,
    };

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(state)
        .expect("Failed to launch application");
}