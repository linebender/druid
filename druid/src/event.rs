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

//! Events.

use crate::kurbo::{Rect, Shape, Size, Vec2};

use druid_shell::{Clipboard, KeyEvent, TimerToken};

use crate::mouse::MouseEvent;
use crate::{Command, Notification, WidgetId};

/// An event, propagated downwards during event flow.
///
/// With two exceptions ([`Event::Command`] and [`Event::Notification`], which
/// have special considerations outlined in their own docs) each event
/// corresponds to some user action or other message recieved from the platform.
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
    /// Sent to all widgets in a given window when that window is first instantiated.
    ///
    /// This should always be the first `Event` received, although widgets will
    /// receive [`LifeCycle::WidgetAdded`] first.
    ///
    /// Widgets should handle this event if they need to do some addition setup
    /// when a window is first created.
    ///
    /// [`LifeCycle::WidgetAdded`]: enum.LifeCycle.html#variant.WidgetAdded
    WindowConnected,
    /// Called on the root widget when the window size changes.
    ///
    /// Discussion: it's not obvious this should be propagated to user
    /// widgets. It *is* propagated through the RootWidget and handled
    /// in the WindowPod, but after that it might be considered better
    /// to just handle it in `layout`.
    WindowSize(Size),
    /// Called when a mouse button is pressed.
    MouseDown(MouseEvent),
    /// Called when a mouse button is released.
    MouseUp(MouseEvent),
    /// Called when the mouse is moved.
    ///
    /// The `MouseMove` event is propagated to the active widget, if
    /// there is one, otherwise to hot widgets (see `HotChanged`).
    /// If a widget loses its hot status due to `MouseMove` then that specific
    /// `MouseMove` event is also still sent to that widget.
    ///
    /// The `MouseMove` event is also the primary mechanism for widgets
    /// to set a cursor, for example to an I-bar inside a text widget. A
    /// simple tactic is for the widget to unconditionally call
    /// [`set_cursor`] in the MouseMove handler, as `MouseMove` is only
    /// propagated to active or hot widgets.
    ///
    /// [`set_cursor`]: struct.EventCtx.html#method.set_cursor
    MouseMove(MouseEvent),
    /// Called when the mouse wheel or trackpad is scrolled.
    Wheel(MouseEvent),
    /// Called when a key is pressed.
    KeyDown(KeyEvent),
    /// Called when a key is released.
    ///
    /// Because of repeat, there may be a number `KeyDown` events before
    /// a corresponding `KeyUp` is sent.
    KeyUp(KeyEvent),
    /// Called when a paste command is received.
    Paste(Clipboard),
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
    /// Called at the beginning of a new animation frame.
    ///
    /// On the first frame when transitioning from idle to animating, `interval`
    /// will be 0. (This logic is presently per-window but might change to
    /// per-widget to make it more consistent). Otherwise it is in nanoseconds.
    ///
    /// The `paint` method will be called shortly after this event is finished.
    /// As a result, you should try to avoid doing anything computationally
    /// intensive in response to an `AnimFrame` event: it might make Druid miss
    /// the monitor's refresh, causing lag or jerky animation.
    AnimFrame(u64),
    /// An event containing a [`Command`] to be handled by the widget.
    ///
    /// [`Command`]s are messages, optionally with attached data, that can
    /// may be generated from a number of sources:
    ///
    /// - If your application uses  menus (either window or context menus)
    /// then the [`MenuItem`]s in the menu will each correspond to a `Command`.
    /// When the menu item is selected, that [`Command`] will be delivered to
    /// the root widget of the appropriate window.
    /// - If you are doing work in another thread (using an [`ExtEventSink`])
    /// then [`Command`]s are the mechanism by which you communicate back to
    /// the main thread.
    /// - Widgets and other Druid components can send custom [`Command`]s at
    /// runtime, via methods such as [`EventCtx::submit_command`].
    ///
    /// [`Command`]: struct.Command.html
    /// [`Widget`]: trait.Widget.html
    /// [`EventCtx::submit_command`]: struct.EventCtx.html#method.submit_command
    /// [`ExtEventSink`]: crate::ExtEventSink
    /// [`MenuItem`]: crate::MenuItem
    Command(Command),
    /// A [`Notification`] from one of this widget's descendants.
    ///
    /// While handling events, widgets can submit notifications to be
    /// delivered to their ancestors immdiately after they return.
    ///
    /// If you handle a [`Notification`], you should call [`EventCtx::set_handled`]
    /// to stop the notification from being delivered to further ancestors.
    ///
    /// ## Special considerations
    ///
    /// Notifications are slightly different from other events; they originate
    /// inside Druid, and they are delivered as part of the handling of another
    /// event. In this sense, they can sort of be thought of as an augmentation
    /// of an event; they are a way for multiple widgets to coordinate the
    /// handling of an event.
    ///
    /// [`EventCtx::set_handled`]: crate::EventCtx::set_handled
    Notification(Notification),
    /// Internal druid event.
    ///
    /// This should always be passed down to descendant [`WidgetPod`]s.
    ///
    /// [`WidgetPod`]: struct.WidgetPod.html
    Internal(InternalEvent),
}

/// Internal events used by druid inside [`WidgetPod`].
///
/// These events are translated into regular [`Event`]s
/// and should not be used directly.
///
/// [`WidgetPod`]: struct.WidgetPod.html
/// [`Event`]: enum.Event.html
#[derive(Debug, Clone)]
pub enum InternalEvent {
    /// Sent in some cases when the mouse has left the window.
    ///
    /// This is used in cases when the platform no longer sends mouse events,
    /// but we know that we've stopped receiving the mouse events.
    MouseLeave,
    /// A command still in the process of being dispatched.
    TargetedCommand(Command),
    /// Used for routing timer events.
    RouteTimer(TimerToken, WidgetId),
}

/// Application life cycle events.
///
/// Unlike [`Event`]s, [`LifeCycle`] events are generated by Druid, and
/// may occur at different times during a given pass of the event loop. The
/// [`LifeCycle::WidgetAdded`] event, for instance, may occur when the app
/// first launches (during the handling of [`Event::WindowConnected`]) or it
/// may occur during [`update`] cycle, if some widget has been added there.
///
/// Similarly the [`LifeCycle::Size`] method occurs during [`layout`], and
/// [`LifeCycle::HotChanged`] can occur both during [`event`] (if the mouse
/// moves over a widget) or during [`layout`], if a widget is resized and
/// that moves it under the mouse.
///
/// [`event`]: crate::Widget::event
/// [`update`]: crate::Widget::update
/// [`layout`]: crate::Widget::layout
#[derive(Debug, Clone)]
pub enum LifeCycle {
    /// Sent to a `Widget` when it is added to the widget tree. This should be
    /// the first message that each widget receives.
    ///
    /// Widgets should handle this event in order to do any initial setup.
    ///
    /// In addition to setup, this event is also used by the framework to
    /// track certain types of important widget state.
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
    WidgetAdded,
    /// Called when the [`Size`] of the widget changes.
    ///
    /// This will be called after [`Widget::layout`], if the [`Size`] returned
    /// by the widget differs from its previous size.
    ///
    /// [`Size`]: struct.Size.html
    /// [`Widget::layout`]: trait.Widget.html#tymethod.layout
    Size(Size),
    /// Called when the "hot" status changes.
    ///
    /// This will always be called _before_ the event that triggered it; that is,
    /// when the mouse moves over a widget, that widget will receive
    /// `LifeCycle::HotChanged` before it receives `Event::MouseMove`.
    ///
    /// See [`is_hot`](struct.EventCtx.html#method.is_hot) for
    /// discussion about the hot status.
    HotChanged(bool),
    /// Called when the focus status changes.
    ///
    /// This will always be called immediately after a new widget gains focus.
    /// The newly focused widget will receive this with `true` and the widget
    /// that lost focus will receive this with `false`.
    ///
    /// See [`EventCtx::is_focused`] for more information about focus.
    ///
    /// [`EventCtx::is_focused`]: struct.EventCtx.html#method.is_focused
    FocusChanged(bool),
    /// Internal druid lifecycle event.
    ///
    /// This should always be passed down to descendant [`WidgetPod`]s.
    ///
    /// [`WidgetPod`]: struct.WidgetPod.html
    Internal(InternalLifeCycle),
}

/// Internal lifecycle events used by druid inside [`WidgetPod`].
///
/// These events are translated into regular [`LifeCycle`] events
/// and should not be used directly.
///
/// [`WidgetPod`]: struct.WidgetPod.html
/// [`LifeCycle`]: enum.LifeCycle.html
#[derive(Debug, Clone)]
pub enum InternalLifeCycle {
    /// Used to route the `WidgetAdded` event to the required widgets.
    RouteWidgetAdded,
    /// Used to route the `FocusChanged` event.
    RouteFocusChanged {
        /// the widget that is losing focus, if any
        old: Option<WidgetId>,
        /// the widget that is gaining focus, if any
        new: Option<WidgetId>,
    },
    /// Testing only: request the `WidgetState` of a specific widget.
    ///
    /// During testing, you may wish to verify that the state of a widget
    /// somewhere in the tree is as expected. In that case you can dispatch
    /// this event, specifying the widget in question, and that widget will
    /// set its state in the provided `Cell`, if it exists.
    #[cfg(test)]
    DebugRequestState {
        widget: WidgetId,
        state_cell: StateCell,
    },
    #[cfg(test)]
    DebugInspectState(StateCheckFn),
}

impl Event {
    /// Transform the event for the contents of a scrolling container.
    ///
    /// the `force` flag is used to ensure an event is delivered even
    /// if the cursor is out of the viewport, such as if the contents are active
    /// or hot.
    pub fn transform_scroll(&self, offset: Vec2, viewport: Rect, force: bool) -> Option<Event> {
        match self {
            Event::MouseDown(mouse_event) => {
                if force || viewport.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseDown(mouse_event))
                } else {
                    None
                }
            }
            Event::MouseUp(mouse_event) => {
                if force || viewport.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseUp(mouse_event))
                } else {
                    None
                }
            }
            Event::MouseMove(mouse_event) => {
                if force || viewport.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::MouseMove(mouse_event))
                } else {
                    None
                }
            }
            Event::Wheel(mouse_event) => {
                if force || viewport.winding(mouse_event.pos) != 0 {
                    let mut mouse_event = mouse_event.clone();
                    mouse_event.pos += offset;
                    Some(Event::Wheel(mouse_event))
                } else {
                    None
                }
            }
            _ => Some(self.clone()),
        }
    }

    /// Whether this event should be sent to widgets which are currently not visible
    /// (for example the hidden tabs in a tabs widget).
    pub fn should_propagate_to_hidden(&self) -> bool {
        match self {
            Event::WindowConnected
            | Event::WindowSize(_)
            | Event::Timer(_)
            | Event::AnimFrame(_)
            | Event::Command(_)
            | Event::Notification(_)
            | Event::Internal(_) => true,
            Event::MouseDown(_)
            | Event::MouseUp(_)
            | Event::MouseMove(_)
            | Event::Wheel(_)
            | Event::KeyDown(_)
            | Event::KeyUp(_)
            | Event::Paste(_)
            | Event::Zoom(_) => false,
        }
    }
}

impl LifeCycle {
    /// Whether this event should be sent to widgets which are currently not visible
    /// (for example the hidden tabs in a tabs widget).
    pub fn should_propagate_to_hidden(&self) -> bool {
        match self {
            LifeCycle::WidgetAdded | LifeCycle::Internal(_) => true,
            LifeCycle::Size(_) | LifeCycle::HotChanged(_) | LifeCycle::FocusChanged(_) => false,
        }
    }
}

#[cfg(test)]
pub(crate) use state_cell::{StateCell, StateCheckFn};

#[cfg(test)]
mod state_cell {
    use crate::core::WidgetState;
    use crate::WidgetId;
    use std::{cell::RefCell, rc::Rc};

    /// An interior-mutable struct for fetching BasteState.
    #[derive(Clone, Default)]
    pub struct StateCell(Rc<RefCell<Option<WidgetState>>>);

    #[derive(Clone)]
    pub struct StateCheckFn(Rc<dyn Fn(&WidgetState)>);

    /// a hacky way of printing the widget id if we panic
    struct WidgetDrop(bool, WidgetId);

    impl Drop for WidgetDrop {
        fn drop(&mut self) {
            if self.0 {
                eprintln!("panic in {:?}", self.1);
            }
        }
    }

    impl StateCell {
        /// Set the state. This will panic if it is called twice.
        pub(crate) fn set(&self, state: WidgetState) {
            assert!(
                self.0.borrow_mut().replace(state).is_none(),
                "StateCell already set"
            )
        }

        #[allow(dead_code)]
        pub(crate) fn take(&self) -> Option<WidgetState> {
            self.0.borrow_mut().take()
        }
    }

    impl StateCheckFn {
        #[cfg(not(target_arch = "wasm32"))]
        pub(crate) fn new(f: impl Fn(&WidgetState) + 'static) -> Self {
            StateCheckFn(Rc::new(f))
        }

        pub(crate) fn call(&self, state: &WidgetState) {
            let mut panic_reporter = WidgetDrop(true, state.id);
            (self.0)(&state);
            panic_reporter.0 = false;
        }
    }

    impl std::fmt::Debug for StateCell {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            let inner = if self.0.borrow().is_some() {
                "Some"
            } else {
                "None"
            };
            write!(f, "StateCell({})", inner)
        }
    }

    impl std::fmt::Debug for StateCheckFn {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "StateCheckFn")
        }
    }
}
