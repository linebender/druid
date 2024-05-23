// Copyright 2023 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! A demo of a few window features, including input region, always on top,
//! and titlebar visibility.
//! The demo is setup so that there are parts of the window that you can click through.
//! There are also buttons for setting always on top and setting the visibility of the
//! titlebar.
//! The window's region is managed by the root custom widget.

use druid::widget::prelude::*;
use druid::widget::{Button, Container, Flex, Label, LineBreaking, Widget};
use druid::{AppLauncher, Color, Lens, Point, Rect, Region, WidgetExt, WidgetPod, WindowDesc};

const EXAMPLE_BORDER_SIZE: f64 = 3.0;
const INFO_TEXT: &str = "Only this text and the borders can be interacted with.
You can click through the other parts

This demo is useful for observing the limitations of each OS.
- Windows is well supported. Observation: When the titlebar is enabled and the input region is set, the border becomes invisible. Always on top is supported.
- macOS has good support, but doesn't allow toggling titlebar after the window is opened. Also, it just makes transparent regions transparent automatically when set to have no titlebar. Always on top is supported.
- Linux support varies by desktop environment and display server. Wayland is much more restrictive, with it not allowing things like setting position and always on top. Fortunately desktop environments often allow you to manually set window decoration and always on top on the Window itself. The offsets can differ between desktop environments, and sometimes you need to open the window with the titlebar, then turn it off, for it to work.";

#[derive(Clone, Data, Lens)]
struct AppState {
    limit_input_region: bool,
    show_titlebar: bool,
    always_on_top: bool,
    mouse_pass_through_while_not_in_focus: bool,
    mouse_pass_through: bool,
}

struct InputRegionExampleWidget {
    info_label: WidgetPod<AppState, Container<AppState>>,
    controls: WidgetPod<AppState, Flex<AppState>>,
}

impl InputRegionExampleWidget {
    pub fn new() -> Self {
        let info_label = Label::new(INFO_TEXT)
            .with_line_break_mode(LineBreaking::WordWrap)
            .padding(20.0)
            .background(Color::rgba(0.2, 0.2, 0.2, 0.5));
        let toggle_input_region = Button::new("Toggle Input Region")
            .on_click(|ctx, data: &mut bool, _: &Env| {
                *data = !*data;
                tracing::debug!("Setting input region toggle to: {}", *data);
                ctx.request_layout();
            })
            .lens(AppState::limit_input_region);
        let toggle_titlebar = Button::new("Toggle TitleBar")
            .on_click(|ctx, data: &mut bool, _: &Env| {
                *data = !*data;
                tracing::debug!("Setting titlebar visibility to: {}", *data);
                ctx.window().show_titlebar(*data);
                ctx.request_layout();
            })
            .lens(AppState::show_titlebar);
        let toggle_always_on_top = Button::new("Toggle Always On Top")
            .on_click(|ctx, data: &mut bool, _: &Env| {
                *data = !*data;
                tracing::debug!("Setting always on top to: {}", *data);
                ctx.window().set_always_on_top(*data);
            })
            .lens(AppState::always_on_top);
        let toggle_mouse_pass_through_while_not_in_focus = Button::new("Toggle Mouse Pass Through")
            .on_click(|_, data: &mut bool, _: &Env| {
                *data = !*data;
                tracing::debug!(
                    "Setting mouse pass through while not in focus to: {}",
                    *data
                );
            })
            .lens(AppState::mouse_pass_through_while_not_in_focus);
        let controls_flex = Flex::row()
            .with_child(toggle_input_region)
            .with_child(toggle_titlebar)
            .with_child(toggle_always_on_top)
            .with_child(toggle_mouse_pass_through_while_not_in_focus);
        Self {
            info_label: WidgetPod::new(info_label),
            controls: WidgetPod::new(controls_flex),
        }
    }
}

impl Widget<AppState> for InputRegionExampleWidget {
    fn event(
        &mut self,
        ctx: &mut druid::EventCtx,
        event: &druid::Event,
        data: &mut AppState,
        env: &druid::Env,
    ) {
        self.info_label.event(ctx, event, data, env);
        self.controls.event(ctx, event, data, env);
        let mouse_pass_through =
            data.mouse_pass_through_while_not_in_focus && !ctx.window().is_foreground_window();
        if mouse_pass_through != data.mouse_pass_through {
            data.mouse_pass_through = mouse_pass_through;
            tracing::debug!("Setting mouse pass through to: {}", mouse_pass_through);
            ctx.window().set_mouse_pass_through(mouse_pass_through);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut druid::LifeCycleCtx,
        event: &druid::LifeCycle,
        data: &AppState,
        env: &druid::Env,
    ) {
        self.info_label.lifecycle(ctx, event, data, env);
        self.controls.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut druid::UpdateCtx,
        _old_data: &AppState,
        data: &AppState,
        env: &druid::Env,
    ) {
        self.info_label.update(ctx, data, env);
        self.controls.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut druid::LayoutCtx,
        bc: &druid::BoxConstraints,
        data: &AppState,
        env: &druid::Env,
    ) -> druid::Size {
        let mut interactable_area = Region::EMPTY;
        let smaller_bc = BoxConstraints::new(
            Size::new(0.0, 0.0),
            Size::new(bc.max().width - 100.0, bc.max().height - 100.0),
        );
        let full_bc = BoxConstraints::new(Size::new(0.0, 0.0), bc.max());
        let _label_size = self.info_label.layout(ctx, &smaller_bc, data, env);
        let controls_size = self.controls.layout(ctx, &full_bc, data, env);

        let text_origin_point = Point::new(50.0, 50.0 + controls_size.height);
        self.info_label.set_origin(ctx, text_origin_point);
        let controls_origin_point = Point::new(EXAMPLE_BORDER_SIZE, EXAMPLE_BORDER_SIZE);
        self.controls.set_origin(ctx, controls_origin_point);

        // Add side rects to clarify the dimensions of the window.
        let left_rect = Rect::new(0.0, 0.0, EXAMPLE_BORDER_SIZE, bc.max().height);
        let right_rect = Rect::new(
            bc.max().width - EXAMPLE_BORDER_SIZE,
            0.0,
            bc.max().width,
            bc.max().height,
        );
        let bottom_rect = Rect::new(
            0.0,
            bc.max().height - EXAMPLE_BORDER_SIZE,
            bc.max().width,
            bc.max().height,
        );
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
        let right_rect = Rect::new(
            window_area.width - EXAMPLE_BORDER_SIZE,
            0.0,
            window_area.width,
            window_area.height,
        );
        let bottom_rect = Rect::new(
            0.0,
            window_area.height - EXAMPLE_BORDER_SIZE,
            window_area.width,
            window_area.height,
        );

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
        .window_size((750.0, 500.0))
        .with_min_size((650.0, 450.0))
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
        mouse_pass_through_while_not_in_focus: false,
        mouse_pass_through: false,
    };

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(state)
        .expect("Failed to launch application");
}
