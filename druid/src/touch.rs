// Copyright 2019 The Druid Authors.
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

//! Structs for touch events

use std::time::Duration;
use std::collections::VecDeque;
use crate::kurbo::Point;

use crate::Modifiers;
pub use crate::shell::TouchSequenceId;

use crate::{Env, Event, EventCtx, TimerToken, MouseButton, MouseButtons, MouseEvent, PointerType, Vec2};

#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub pos: Point,
    pub window_pos: Point,
    pub mods: Modifiers,
    pub focus: bool,
    pub sequence_id: Option<TouchSequenceId>,
}

impl From<druid_shell::TouchEvent> for TouchEvent {
    fn from(src: druid_shell::TouchEvent) -> TouchEvent {
        let druid_shell::TouchEvent {
            pos,
            mods,
            focus,
            sequence_id,
        } = src;
        TouchEvent {
            pos,
            window_pos: pos,
            mods,
            focus,
            sequence_id,
        }
    }
}

const TOUCH_IS_CLICK: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct TouchProcessor {
    timer_token: TimerToken,
    event_count: u32,
    touch_ended: bool,
    touch_begin: Option<TouchEvent>,
    processed_events: VecDeque<Event>,
    is_dragging: bool,
}

impl TouchProcessor {
    pub fn new() -> TouchProcessor {
        TouchProcessor {
            timer_token: TimerToken::INVALID,
            event_count: 0,
            touch_ended: false,
            touch_begin: None,
            processed_events: VecDeque::new(),
            is_dragging: false,
        }
    }

    pub fn handle(&mut self, ctx: &mut EventCtx, event: &Event, env: &Env) {
        match event {
            Event::TouchBegin(touch_event) => {
                // TODO: track sequence
                self.touch_ended = false;
                self.event_count = 0;
                self.touch_begin = Some(touch_event.clone());
                self.timer_token = ctx.request_timer(TOUCH_IS_CLICK);
                self.is_dragging = false;
            },
            Event::TouchUpdate(touch_event) => {
                self.event_count += 1;

                if self.is_dragging {
                    self.processed_events.push_back(Event::Wheel(MouseEvent {
                        pos: touch_event.pos,
                        window_pos: touch_event.window_pos,
                        button: MouseButton::None,
                        buttons: MouseButtons::new(),
                        mods: touch_event.mods,
                        count: 0,
                        focus: touch_event.focus,
                        wheel_delta: self.touch_begin.as_ref().unwrap().pos - touch_event.pos,
                        pointer_type: PointerType::Touch,
                    }));

                    self.touch_begin = Some(touch_event.clone());
                }
            },
            Event::TouchEnd(_) => {
                self.touch_ended = true;
            },
            Event::Timer(token) => {
                if token == &self.timer_token {
                    self.is_dragging = !self.touch_ended;
                    if self.touch_ended {
                        let touch_event = self.touch_begin.as_ref().unwrap();
                        self.processed_events.push_back(Event::MouseDown(MouseEvent {
                            pos: touch_event.pos,
                            window_pos: touch_event.window_pos,
                            button: MouseButton::Left,
                            buttons: MouseButtons::new(),
                            mods: touch_event.mods,
                            count: 0,
                            focus: touch_event.focus,
                            wheel_delta: Vec2::ZERO,
                            pointer_type: PointerType::Touch,
                        }));
                        self.processed_events.push_back(Event::MouseUp(MouseEvent {
                            pos: touch_event.pos,
                            window_pos: touch_event.window_pos,
                            button: MouseButton::Left,
                            buttons: MouseButtons::new(),
                            mods: touch_event.mods,
                            count: 0,
                            focus: touch_event.focus,
                            wheel_delta: Vec2::ZERO,
                            pointer_type: PointerType::Touch,
                        }));
                    }
                }
            },
            _ => {},
        }
    }

    pub fn processed_events(&mut self) -> VecDeque<Event> {
        std::mem::replace(&mut self.processed_events, VecDeque::new())
    }
}
