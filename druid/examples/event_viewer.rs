// Copyright 2020 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! An application that accepts keyboard and mouse input, and displays
//! information about received events.

use druid::widget::prelude::*;
use druid::widget::{Controller, CrossAxisAlignment, Flex, Label, List, Scroll, SizedBox, TextBox};
use druid::{
    theme, AppLauncher, Color, Data, FontDescriptor, KeyEvent, Lens, Location, Modifiers,
    MouseButton, MouseEvent, WidgetExt, WindowDesc,
};
use std::sync::Arc;

const CURSOR_BACKGROUND_COLOR: Color = Color::grey8(0x55);
const HEADER_BACKGROUND: Color = Color::grey8(0xCC);
const INTERACTIVE_AREA_DIM: f64 = 160.0;
const INTERACTIVE_AREA_BORDER: Color = Color::grey8(0xCC);
const TEXT_COLOR: Color = Color::grey8(0x11);
const PROPERTIES: &[(&str, f64)] = &[
    ("#", 40.0),
    ("Event", 80.0),
    ("Point", 90.0),
    ("Wheel", 80.0),
    ("Button", 60.0),
    ("Count", 50.0),
    ("Repeat", 50.0),
    ("Key", 60.0),
    ("Code", 60.0),
    ("Modifiers", 80.0),
    ("Location", 60.0),
];

#[allow(clippy::clippy::rc_buffer)]
#[derive(Clone, Data, Lens)]
struct AppState {
    /// The text in the text field
    text_input: String,
    events: Arc<Vec<LoggedEvent>>,
}

struct EventLogger<F: Fn(&Event) -> bool> {
    filter: F,
}

impl<W: Widget<AppState>, F: Fn(&Event) -> bool> Controller<AppState, W> for EventLogger<F> {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        // Every time this controller receives an event we check `f()`.
        // If `f()` returns true it means that we can add it to the log,
        // if not then we can skip it.
        if (self.filter)(event) {
            if let Some(to_log) = LoggedEvent::try_from_event(event, data.events.len()) {
                Arc::make_mut(&mut data.events).push(to_log);
            }
        }
        // Always pass on the event!
        child.event(ctx, event, data, env)
    }
}

/// The types of events we display
#[derive(Clone, Copy, Data, PartialEq)]
enum EventType {
    KeyDown,
    KeyUp,
    MouseDown,
    MouseUp,
    Wheel,
}

/// A type that represents any logged event shown in the list
#[derive(Clone, Data)]
struct LoggedEvent {
    typ: EventType,
    number: usize,
    // To see what #[data(ignore)] does look at the docs.rs page on `Data`:
    // https://docs.rs/druid/0.6.0/druid/trait.Data.html
    #[data(ignore)]
    mouse: Option<MouseEvent>,
    #[data(ignore)]
    key: Option<KeyEvent>,
}

/// Here we implement all the display elements of the log entry.
/// We have one method for every attribute we want to show.
/// This is not very interesting it is mostly just getting the data
/// from the events and handling `None` values.
impl LoggedEvent {
    fn try_from_event(event: &Event, number: usize) -> Option<Self> {
        let to_log = match event {
            Event::MouseUp(mouse) => Some((EventType::MouseUp, Some(mouse.clone()), None)),
            Event::MouseDown(mouse) => Some((EventType::MouseDown, Some(mouse.clone()), None)),
            Event::Wheel(mouse) => Some((EventType::Wheel, Some(mouse.clone()), None)),
            Event::KeyUp(key) => Some((EventType::KeyUp, None, Some(key.clone()))),
            Event::KeyDown(key) => Some((EventType::KeyDown, None, Some(key.clone()))),
            _ => None,
        };

        to_log.map(|(typ, mouse, key)| LoggedEvent {
            typ,
            number,
            mouse,
            key,
        })
    }

    fn number(&self) -> String {
        self.number.to_string()
    }

    fn name(&self) -> String {
        match self.typ {
            EventType::KeyDown => "KeyDown",
            EventType::KeyUp => "KeyUp",
            EventType::MouseDown => "MouseDown",
            EventType::MouseUp => "MouseUp",
            EventType::Wheel => "Wheel",
        }
        .into()
    }

    fn mouse_pos(&self) -> String {
        self.mouse
            .as_ref()
            .map(|m| format!("{:.2}", m.pos))
            .unwrap_or_default()
    }

    fn wheel_delta(&self) -> String {
        self.mouse
            .as_ref()
            .filter(|_| self.typ == EventType::Wheel)
            .map(|m| format!("({:.1}, {:.1})", m.wheel_delta.x, m.wheel_delta.y))
            .unwrap_or_default()
    }

    fn mouse_button(&self) -> String {
        self.mouse
            .as_ref()
            .map(|m| {
                match m.button {
                    MouseButton::Left => "Left",
                    MouseButton::Right => "Right",
                    MouseButton::X1 => "X1",
                    MouseButton::X2 => "X2",
                    MouseButton::None => "",
                    MouseButton::Middle => "Middle",
                }
                .into()
            })
            .unwrap_or_default()
    }

    fn click_count(&self) -> String {
        self.mouse
            .as_ref()
            .map(|m| m.count.to_string())
            .unwrap_or_default()
    }

    fn key(&self) -> String {
        self.key
            .as_ref()
            .map(|k| k.key.to_string())
            .unwrap_or_default()
    }

    fn code(&self) -> String {
        self.key
            .as_ref()
            .map(|k| k.code.to_string())
            .unwrap_or_default()
    }

    fn modifiers(&self) -> String {
        self.key
            .as_ref()
            .map(|k| modifiers_string(k.mods))
            .or_else(|| self.mouse.as_ref().map(|m| modifiers_string(m.mods)))
            .unwrap_or_default()
    }

    fn location(&self) -> String {
        match self.key.as_ref().map(|k| k.location) {
            None => "",
            Some(Location::Standard) => "Standard",
            Some(Location::Left) => "Left",
            Some(Location::Right) => "Right",
            Some(Location::Numpad) => "Numpad",
        }
        .into()
    }

    fn is_repeat(&self) -> String {
        if self.key.as_ref().map(|k| k.repeat).unwrap_or(false) {
            "True".to_string()
        } else {
            "False".to_string()
        }
    }
}

fn build_root_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(interactive_area())
        .with_flex_child(event_list(), 1.0)
}

/// The top part of the application, that accepts keyboard and mouse input.
fn interactive_area() -> impl Widget<AppState> {
    let text_box = TextBox::multiline()
        .with_text_color(Color::rgb8(0xf0, 0xf0, 0xea))
        .fix_size(INTERACTIVE_AREA_DIM, INTERACTIVE_AREA_DIM)
        .lens(AppState::text_input)
        .controller(EventLogger {
            filter: |event| matches!(event, Event::KeyDown(_) | Event::KeyUp(_)),
        });

    let mouse_box = SizedBox::empty()
        .fix_size(INTERACTIVE_AREA_DIM, INTERACTIVE_AREA_DIM)
        .background(CURSOR_BACKGROUND_COLOR)
        .rounded(5.0)
        .border(INTERACTIVE_AREA_BORDER, 1.0)
        .controller(EventLogger {
            filter: |event| matches!(event, Event::MouseDown(_) | Event::MouseUp(_) | Event::Wheel(_)),
		});

    Flex::row()
        .with_flex_spacer(1.0)
        .with_child(text_box)
        .with_flex_spacer(1.0)
        .with_child(mouse_box)
        .with_flex_spacer(1.0)
        .padding(10.0)
}

/// The bottom part of the application, a list of received events.
fn event_list() -> impl Widget<AppState> {
    // Because this would be a HUGE block of repeated code with constants
    // we just use a loop to generate the header.
    let mut header = Flex::row().with_child(
        Label::new(PROPERTIES[0].0)
            .fix_width(PROPERTIES[0].1)
            .background(HEADER_BACKGROUND),
    );

    for (name, size) in PROPERTIES.iter().skip(1) {
        // Keep in mind that later on, in the main function,
        // we set the default spacer values. Without explicitly
        // setting them the default spacer is bigger, and is
        // probably not desirable for your purposes.
        header.add_default_spacer();
        header.add_child(
            Label::new(*name)
                .fix_width(*size)
                .background(HEADER_BACKGROUND),
        );
    }
    Scroll::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .with_child(header)
            .with_default_spacer()
            .with_flex_child(
                // `List::new` generates a list entry for every element in the `Vec`.
                // In this case it shows a log entry for every element in `AppState::events`.
                // `make_list_item` generates this new log entry.
                Scroll::new(List::new(make_list_item).lens(AppState::events)).vertical(),
                1.0,
            )
            .background(Color::WHITE),
    )
    .horizontal()
    .padding(10.0)
}

/// A single event row.
fn make_list_item() -> impl Widget<LoggedEvent> {
    Flex::row()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.number()).fix_width(PROPERTIES[0].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.name()).fix_width(PROPERTIES[1].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.mouse_pos()).fix_width(PROPERTIES[2].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.wheel_delta()).fix_width(PROPERTIES[3].1))
        .with_default_spacer()
        .with_child(
            Label::dynamic(|d: &LoggedEvent, _| d.mouse_button()).fix_width(PROPERTIES[4].1),
        )
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.click_count()).fix_width(PROPERTIES[5].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.is_repeat()).fix_width(PROPERTIES[6].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.key()).fix_width(PROPERTIES[7].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.code()).fix_width(PROPERTIES[8].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.modifiers()).fix_width(PROPERTIES[9].1))
        .with_default_spacer()
        .with_child(Label::dynamic(|d: &LoggedEvent, _| d.location()).fix_width(PROPERTIES[10].1))
}

pub fn main() {
    //describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title("Event Viewer")
        .window_size((760.0, 680.0));

    //start the application
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .configure_env(|env, _| {
            env.set(theme::UI_FONT, FontDescriptor::default().with_size(12.0));
            env.set(theme::LABEL_COLOR, TEXT_COLOR);
            env.set(theme::WIDGET_PADDING_HORIZONTAL, 2.0);
            env.set(theme::WIDGET_PADDING_VERTICAL, 2.0);
        })
        .launch(AppState {
            text_input: String::new(),
            events: Arc::new(Vec::new()),
        })
        .expect("Failed to launch application");
}

fn modifiers_string(mods: Modifiers) -> String {
    let mut result = String::new();
    if mods.shift() {
        result.push_str("Shift ");
    }
    if mods.ctrl() {
        result.push_str("Ctrl ");
    }
    if mods.alt() {
        result.push_str("Alt ");
    }
    if mods.meta() {
        result.push_str("Meta ");
    }
    if result.is_empty() {
        result.push_str("None");
    }
    result
}
