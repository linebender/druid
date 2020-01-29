// Copyright 2019 The xi-editor Authors.
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

//! Events.

use crate::kurbo::{Rect, Shape, Size, Vec2};

use druid_shell::{Clipboard, KeyEvent, KeyModifiers, TimerToken};

use crate::mouse::MouseEvent;
use crate::{Command, Target, WidgetId};

/// An event, propagated downwards during event flow.
///
/// Events are things that happen that can change the state of widgets.
/// An important category is events plumbed from the platform windowing
/// system, which includes mouse and keyboard events, but also (in the
/// future) status changes such as window focus changes.
///
/// Events can also be higher level concepts indicating state changes
/// within the widget hierarchy, for example when a widget gains or loses
/// focus or "hot" (also known as hover) status.
///
/// Events are a key part of what is called "event flow", which is
/// basically the propagation of an event through the widget hierarchy
/// through the [`event`] widget method. A container widget will
/// generally pass the event to its children, mediated through the
/// [`WidgetPod`] container, which is where most of the event flow logic
/// is applied (especially the decision whether or not to propagate).
///
/// This enum is expected to grow considerably, as there are many, many
/// different kinds of events that are relevant in a GUI.
///
/// [`event`]: trait.Widget.html#tymethod.event
/// [`WidgetPod`]: struct.WidgetPod.html
#[derive(Debug, Clone)]
pub enum Event {
    /// Called on the root widget when the window size changes.
    ///
    /// Discussion: it's not obvious this should be propagated to user
    /// widgets. It *is* propagated through the RootWidget and handled
    /// in the WindowPod, but after that it might be considered better
    /// to just handle it in `layout`.
    ///
    /// The propagation logic of "just the root" requires a little bit
    /// of complexity and state in EventCtx, so if it's not useful it
    /// should be removed.
    Size(Size),
    /// Called when a mouse button is pressed.
    MouseDown(MouseEvent),
    /// Called when a mouse button is released.
    MouseUp(MouseEvent),
    /// Called when the mouse is moved.
    ///
    /// The `MouseMoved` event is propagated to the active widget, if
    /// there is one, otherwise to hot widgets (see `HotChanged`).
    ///
    /// The `MouseMoved` event is also the primary mechanism for widgets
    /// to set a cursor, for example to an I-bar inside a text widget. A
    /// simple tactic is for the widget to unconditionally call
    /// [`set_cursor`] in the MouseMoved handler, as `MouseMove` is only
    /// propagated to active or hot widgets.
    ///
    /// [`set_cursor`]: struct.EventCtx.html#method.set_cursor
    MouseMoved(MouseEvent),
    /// Called when a key is pressed.
    ///
    /// Note: the intent is for each physical key press to correspond to
    /// a single `KeyDown` event. This is sometimes different than the
    /// raw events provided by the platform. In particular, Windows sends
    /// one or both of WM_KEYDOWN (a raw key code) and WM_CHAR (the
    /// Unicode value), depending on the actual key.
    KeyDown(KeyEvent),
    /// Called when a key is released.
    ///
    /// Because of repeat, there may be a number `KeyDown` events before
    /// a corresponding `KeyUp` is sent.
    KeyUp(KeyEvent),
    /// Called when a paste command is received.
    Paste(Clipboard),
    /// Called when the mouse wheel or trackpad is scrolled.
    Wheel(WheelEvent),
    /// Called when the trackpad is pinched.
    ///
    /// The value is a delta.
    Zoom(f64),
    /// Called on a timer event.
    ///
    /// Request a timer event through [`EventCtx::request_timer()`]. That will
    /// cause a timer event later.
    ///
    /// Note that timer events from other widgets may be delivered as well. Use
    /// the token returned from the `request_timer()` call to filter events more
    /// precisely.
    ///
    /// [`EventCtx::request_timer()`]: struct.EventCtx.html#method.request_timer
    Timer(TimerToken),
    /// Called with an arbitrary [`Command`], submitted from elsewhere in
    /// the application.
    ///
    /// Commands can be issued when the user triggers a menu item or an
    /// application-level hotkey, or they can be created dynamically by
    /// [`Widget`]s, at runtime, with [`EventCtx::submit_command`].
    ///
    /// [`Command`]: struct.Command.html
    /// [`Widget`]: trait.Widget.html
    /// [`EventCtx::submit_command`]: struct.EventCtx.html#method.submit_command
    Command(Command),
    /// A command still in the process of being dispatched. This is an internal
    /// event and should generally not be handled directly by widgets, but is
    /// important for containers to dispatch to their children.
    TargetedCommand(Target, Command),
}

/// Application life cycle events.
#[derive(Debug, Clone, Copy)]
pub enum LifeCycle {
    /// Sent to a `Widget` when it is added to the widget tree. This should be
    /// the first message that each widget receives.
    ///
    /// Widgets should handle this event if they need to do any one-time setup
    /// that requires access to `Data`.
    WidgetAdded,
    /// Sent to all widgets in a given window when that window is first instantiated.
    ///
    /// This is sent after `WidgetAdded`. Widgets should handle this event if
    /// they need to do some addition setup when a window is first created.
    WindowConnected,
    /// Sent to a widget when it or any of its children have changed.
    ///
    /// This event is used to register widgets with the framework, as well
    /// as to register widgets to participate in certain framework features.
    ///
    /// ## Registering children
    ///
    /// Container widgets (widgets which use [`WidgetPod`] to manage children)
    /// must ensure that this event is forwarded to those children. The [`WidgetPod`]
    /// itself will handle registering those children with the system; this is
    /// required for things like correct routing of events.
    ///
    /// ## Participating in focus
    ///
    /// Widgets which wish to participate in automatic focus (using tab to change
    /// focus) must handle this event and call [`LifeCycleCtx::register_for_focus`].
    ///
    /// [`LifeCycleCtx::register_child`]: struct.LifeCycleCtx.html#method.register_child
    /// [`WidgetPod`]: struct.WidgetPod.html
    /// [`LifeCycleCtx::register_for_focus`]: struct.LifeCycleCtx.html#method.register_for_focus
    Register,
    /// Called at the beginning of a new animation frame.
    ///
    /// On the first frame when transitioning from idle to animating, `interval`
    /// will be 0. (This logic is presently per-window but might change to
    /// per-widget to make it more consistent). Otherwise it is in nanoseconds.
    AnimFrame(u64),
    /// Called when the "hot" status changes.
    ///
    /// This will always be called _before_ the event that triggered it; that is,
    /// when the mouse moves over a widget, that widget will receive
    /// `LifeCycle::HotChanged` before it receives `Event::MouseMoved`.
    ///
    /// See [`is_hot`](struct.BaseState.html#method.is_hot) for
    /// discussion about the hot status.
    HotChanged(bool),
    /// Internal: used by the framework to route the `FocusChanged` event.
    RouteFocusChanged {
        /// the widget that is losing focus, if any
        old: Option<WidgetId>,
        /// the widget that is gaining focus, if any
        new: Option<WidgetId>,
    },
    /// Called when the focus status changes.
    ///
    /// This will always be called immediately after an event where a widget
    /// has requested focus.
    ///
    /// See [`has_focus`](struct.BaseState.html#method.has_focus) for
    /// discussion about the focus status.
    FocusChanged(bool),
}

/// A mouse wheel event.
///
/// An event generated by a mouse wheel or trackpad device. Perhaps a
/// better name would have been "ScrollEvent", but we follow the lead
/// of the W3C in naming, also partly to emphasize that this represents
/// an event from the device, as opposed to a particular intended action.
/// For example, in many cases a wheel event might cause a zoom.
///
/// See the
/// [wiki](https://linebender.gitbook.io/linebender-graphics-wiki/mouse-wheel)
/// for more discussion, including testing on various platforms.
#[derive(Debug, Clone)]
pub struct WheelEvent {
    /// The wheel movement.
    ///
    /// The polarity is the amount to be added to the scroll position,
    /// in other words the opposite of the direction the content should
    /// move on scrolling. This polarity is consistent with the
    /// deltaX and deltaY values in a web [WheelEvent].
    ///
    /// [WheelEvent]: https://w3c.github.io/uievents/#event-type-wheel
    pub delta: Vec2,
    /// The keyboard modifiers at the time of the event.
    pub mods: KeyModifiers,
}

impl Event {
    /// Transform the event for the contents of a scrolling container.
    pub fn transform_scroll(&self, offset: Vec2, viewport: Rect) -> Option<Event> {
        // TODO: need to wire this up so that it always propagates mouse events
        // if the widget is active.
        match self {
            Event::MouseDown(mouse_event) => {
                if viewport.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseDown(mouse_event))
                } else {
                    None
                }
            }
            Event::MouseUp(mouse_event) => {
                if viewport.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseUp(mouse_event))
                } else {
                    None
                }
            }
            Event::MouseMoved(mouse_event) => {
                if viewport.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseMoved(mouse_event))
                } else {
                    None
                }
            }
            _ => Some(self.clone()),
        }
    }
}
