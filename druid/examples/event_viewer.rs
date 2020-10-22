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

use druid::widget::{
    prelude::*, Controller, CrossAxisAlignment, Flex, Label, List, Painter, Scroll, SizedBox,
    TextBox,
};
use druid::{
    theme, AppLauncher, Color, Data, Env, FontDescriptor, KeyEvent, Lens, LocalizedString,
    Location, Modifiers, MouseButton, MouseEvent, Widget, WidgetExt, WindowDesc,
};
use std::sync::Arc;

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Event Viewer");
const INACTIVE_AREA_COLOR: Color = Color::grey8(0x55);
const HOVER_AREA_COLOR: Color = Color::grey8(0xAA);
const ACTIVE_AREA_COLOR: Color = Color::grey8(0xCC);
const HEADER_BACKGROUND: Color = Color::grey8(0xCC);
const COLUMN_PADDING: f64 = 2.0;
const INTERACTIVE_AREA_DIM: f64 = 160.0;

#[derive(Clone, Data, Lens)]
struct AppState {
    /// The text in the text field
    text_input: String,
    events: Arc<Vec<EventLog>>,
    total_events: usize,
}

pub fn main() {
    //describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title(WINDOW_TITLE)
        .window_size((760.0, 680.0));

    //create the initial app state
    let initial_state = AppState {
        text_input: String::new(),
        events: Arc::new(Vec::new()),
        total_events: 0,
    };

    //start the application
    AppLauncher::with_window(main_window)
        .configure_env(|env, _| {
            env.set(theme::UI_FONT, FontDescriptor::default().with_size(12.0));
            env.set(theme::LABEL_COLOR, Color::grey8(0x11));
        })
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    Flex::column()
        .with_child(interactive_area())
        .with_flex_child(event_list(), 1.0)
}

/// The top part of the application, that accepts keyboard and mouse input.
fn interactive_area() -> impl Widget<AppState> {
    Flex::row()
        .with_flex_spacer(0.5)
        .with_child(
            TextBox::multiline()
                .with_text_color(Color::rgb8(0xf0, 0xf0, 0xea))
                .lens(AppState::text_input)
                .controller(EventLogger::new(
                    |event| matches!(event, Event::KeyDown(_) | Event::KeyUp(_)),
                ))
                .fix_size(INTERACTIVE_AREA_DIM, INTERACTIVE_AREA_DIM)
        )
        .with_flex_spacer(0.5)
        .with_child(
            SizedBox::empty()
                .fix_width(INTERACTIVE_AREA_DIM)
                .fix_height(INTERACTIVE_AREA_DIM)
                .background(
                Painter::new(|ctx, _, _env| {
                    let bg_color = if ctx.is_active() {
                        ACTIVE_AREA_COLOR
                    } else if ctx.is_hot() {
                        HOVER_AREA_COLOR
                    } else {
                        INACTIVE_AREA_COLOR
                    };
                    let rect = ctx.size().to_rect();
                    ctx.fill(rect, &bg_color);
                }))
                .rounded(5.0)
                .border(Color::grey8(0xCC), 1.0)
                // an empty on-click handler just so we get hot/active changes in painter
                .on_click(|_, _, _|{})
                .controller(EventLogger::new(
                    |event| matches!(event, Event::MouseDown(_) | Event::MouseUp(_) | Event::Wheel(_)),
                ))
        )
        .with_flex_spacer(0.5)
        .padding(10.0)
}

/// The bottom part of the application, a list of received events.
fn event_list() -> impl Widget<AppState> {
    Flex::column()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_child(
            Flex::row()
                .with_child(
                    Label::new("#")
                        .fix_width(40.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Event")
                        .fix_width(80.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Point")
                        .fix_width(90.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Wheel")
                        .fix_width(80.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Button")
                        .fix_width(60.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Count")
                        .fix_width(50.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Repeat")
                        .fix_width(50.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Key")
                        .fix_width(60.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Code")
                        .fix_width(60.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Modifiers")
                        .fix_width(80.0)
                        .background(HEADER_BACKGROUND),
                )
                .with_spacer(COLUMN_PADDING)
                .with_child(
                    Label::new("Location")
                        .fix_width(60.0)
                        .background(HEADER_BACKGROUND),
                ),
        )
        .with_spacer(COLUMN_PADDING)
        .with_flex_child(
            Scroll::new(List::new(make_list_item).lens(AppState::events)).expand_width(),
            1.0,
        )
        .padding(10.0)
        .background(Color::WHITE)
}

/// A single event row.
fn make_list_item() -> Box<dyn Widget<EventLog>> {
    Box::new(
        Flex::row()
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.number.to_string())
                    .with_text_size(12.0)
                    // this is very hacky; we just hard code some widths
                    // that work.
                    .fix_width(40.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.name())
                    .with_text_size(12.0)
                    .fix_width(80.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.mouse_pos())
                    .with_text_size(12.0)
                    .fix_width(90.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.wheel_delta())
                    .with_text_size(12.0)
                    .fix_width(80.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.mouse_button())
                    .with_text_size(12.0)
                    .fix_width(60.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.click_count())
                    .with_text_size(12.0)
                    .fix_width(50.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.is_repeat())
                    .with_text_size(12.0)
                    .fix_width(50.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.key())
                    .with_text_size(12.0)
                    .fix_width(60.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.code())
                    .with_text_size(12.0)
                    .fix_width(60.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.modifiers())
                    .with_text_size(12.0)
                    .fix_width(80.0),
            )
            .with_spacer(COLUMN_PADDING)
            .with_child(
                Label::dynamic(|d: &EventLog, _| d.location())
                    .with_text_size(12.0)
                    .fix_width(60.0),
            ),
    )
}

/// The types of events we display
#[derive(Debug, Clone, Copy, Data, PartialEq)]
enum EventType {
    KeyDown,
    KeyUp,
    MouseDown,
    MouseUp,
    Wheel,
}

/// A type that represents any logged event.
#[derive(Debug, Clone, Data)]
struct EventLog {
    typ: EventType,
    number: usize,
    #[data(ignore)]
    mouse: Option<MouseEvent>,
    #[data(ignore)]
    key: Option<KeyEvent>,
}

impl EventLog {
    fn try_from_event(event: &Event, number: usize) -> Option<Self> {
        let to_log = match event {
            Event::MouseUp(mouse) => Some((EventType::MouseUp, Some(mouse.clone()), None)),
            Event::MouseDown(mouse) => Some((EventType::MouseDown, Some(mouse.clone()), None)),
            Event::Wheel(mouse) => Some((EventType::Wheel, Some(mouse.clone()), None)),
            Event::KeyUp(key) => Some((EventType::KeyUp, None, Some(key.clone()))),
            Event::KeyDown(key) => Some((EventType::KeyDown, None, Some(key.clone()))),
            _ => None,
        };

        to_log.map(|(typ, mouse, key)| EventLog {
            typ,
            number,
            mouse,
            key,
        })
    }

    fn name(&self) -> String {
        match self.typ {
            EventType::KeyDown => "KeyDown",
            EventType::KeyUp => "KeyUp",
            EventType::MouseDown => "MouseDown",
            EventType::MouseUp => "MouseUp",
            EventType::Wheel => "Wheel",
        }
        .to_string()
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
            .map(|m| mouse_button_string(m.button))
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
        let mods = self
            .key
            .as_ref()
            .map(|k| k.mods)
            .or_else(|| self.mouse.as_ref().map(|m| m.mods))
            .unwrap();
        modifiers_string(mods)
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

/// A controller that logs events that match a predicate.
struct EventLogger {
    filter: Box<dyn Fn(&Event) -> bool>,
}

impl EventLogger {
    /// Create a new `EventLogger`.
    ///
    /// The logger will attempt to log events for with `f` returns `true`.
    fn new(f: impl Fn(&Event) -> bool + 'static) -> Self {
        EventLogger {
            filter: Box::new(f),
        }
    }
}

impl<W: Widget<AppState>> Controller<AppState, W> for EventLogger {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        if (self.filter)(event) {
            data.log_event(event);
        }
        child.event(ctx, event, data, env)
    }
}

impl AppState {
    fn log_event(&mut self, event: &Event) {
        if let Some(to_log) = EventLog::try_from_event(event, self.total_events) {
            Arc::make_mut(&mut self.events).push(to_log);
            self.total_events += 1;
        }
    }
}

fn modifiers_string(mods: Modifiers) -> String {
    let mut result = String::new();
    if mods.shift() {
        result.push_str("Shift");
    }
    if mods.ctrl() {
        result.push_str("Ctrl");
    }
    if mods.alt() {
        result.push_str("Alt");
    }
    if mods.meta() {
        result.push_str("Meta");
    }
    if result.is_empty() {
        "None".into()
    } else {
        result
    }
}

fn mouse_button_string(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "Left",
        MouseButton::Right => "Right",
        MouseButton::X1 => "X1",
        MouseButton::X2 => "X2",
        MouseButton::None => "None",
        MouseButton::Middle => "Middle",
    }
    .into()
}
