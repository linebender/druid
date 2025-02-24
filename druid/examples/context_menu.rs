// Copyright 2025 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Context menu example to increment/decrement a counter 

use druid::widget::prelude::*;
use druid::widget::{Align, Controller, Label};
use druid::{
    AppLauncher, Data, Env, Lens, LocalizedString, Menu, MenuItem, Selector, Widget, WidgetExt,
    WindowDesc,
};

const WINDOW_TITLE: LocalizedString<CounterState> = LocalizedString::new("Context Menu Example");

#[derive(Clone, Data, Lens)]
struct CounterState {
    i: i64,
}

struct CounterController;

const CONTEXT_MENU_COUNTER_INCREMENT: Selector = Selector::new("context-menu-counter-increment");
const CONTEXT_MENU_COUNTER_DECREMENT: Selector = Selector::new("context-menu-counter-decrement");

impl<W: Widget<CounterState>> Controller<CounterState, W> for CounterController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut CounterState,
        env: &Env,
    ) {
        match event {
            Event::MouseUp(event) => {
                if event.button.is_right() || (event.button.is_left() && event.mods.ctrl()) {
                    ctx.show_context_menu::<CounterState>(
                        Menu::new("Counter")
                            .entry(
                                MenuItem::new("Increment").command(CONTEXT_MENU_COUNTER_INCREMENT),
                            )
                            .entry(
                                MenuItem::new("Decrement").command(CONTEXT_MENU_COUNTER_DECREMENT),
                            ),
                        event.pos,
                    );
                }
            }
            Event::Command(command) => {
                if command.is(CONTEXT_MENU_COUNTER_INCREMENT) {
                    data.i += 1;
                } else if command.is(CONTEXT_MENU_COUNTER_DECREMENT) {
                    data.i -= 1;
                }
            }
            _ => {}
        }
        // Always pass on the event!
        child.event(ctx, event, data, env)
    }
}

fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = CounterState { i: 0 };

    // start the application
    AppLauncher::with_window(main_window)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<CounterState> {
    // a label that will determine its text based on the current app data.
    let label = Label::new(|data: &CounterState, _env: &Env| format!("Counter: {}", data.i));

    // center the a widget in the available space
    Align::centered(label).controller(CounterController)
}
