// Copyright 2020 The druid Authors.
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

//! A focus widget.

use druid::widget::prelude::*;

use druid::{commands, Data, FocusNode, HotKey, KbKey, SysMods, Widget, WidgetPod};

/// Controller that allows a widget to be a focusable
/// without wrapping it into [`Focus`] widget.
#[derive(Debug, Clone)]
pub struct FocusController {
    focus_node: FocusNode,
    requested_focus: bool,
    /// Whether a widget should be automatically focused.
    pub auto_focus: bool,
}

impl FocusController {
    /// Create a new [`FocusController`].
    pub fn new() -> Self {
        Self {
            focus_node: FocusNode::empty(),
            requested_focus: false,
            auto_focus: false,
        }
    }

    /// Get focus_node of the controller.
    pub fn focus_node(&self) -> FocusNode {
        self.focus_node
    }

    /// Handle focus events like focus_next or focus_prev and handle focus commands.
    pub fn event(&mut self, ctx: &mut EventCtx, event: &Event) {
        match event {
            Event::KeyDown(key_event) if !ctx.is_handled => {
                match key_event {
                    // Tab and shift+tab
                    k_e if HotKey::new(None, KbKey::Tab).matches(k_e) => {
                        ctx.focus_next();
                    }
                    k_e if HotKey::new(SysMods::Shift, KbKey::Tab).matches(k_e) => {
                        ctx.focus_prev();
                    }
                    _ => (),
                };
            }
            Event::Command(cmd) if cmd.is(commands::REQUEST_FOCUS) => {
                let widget_id = *cmd.get_unchecked(commands::REQUEST_FOCUS);

                if widget_id == ctx.widget_id() {
                    ctx.request_focus();
                }
            }
            Event::Command(cmd) if cmd.is(commands::NEXT_FOCUS) => {
                if self.focus_node.is_focused {
                    ctx.focus_next();
                }
            }
            Event::Command(cmd) if cmd.is(commands::PREV_FOCUS) => {
                if self.focus_node.is_focused {
                    ctx.focus_prev();
                }
            }
            _ => (),
        }
    }

    /// Handle focusable widget lifecycle. Register widget as focusable and
    /// put it into the current focus scope.
    pub fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        match event {
            LifeCycle::WidgetAdded => {
                self.focus_node.widget_id = Some(ctx.widget_id());
                ctx.set_focus_node(self.focus_node);

                // Register widget for focus
                let focus_scope = ctx.focus_scope();
                let focus_scope_widget_id = focus_scope
                    .widget_id
                    .expect("Focusable widget can't be outside FocusScope");
                let widget_id = ctx.widget_id();
                let focus_chain = ctx
                    .widget_state
                    .focus_chains
                    .entry(focus_scope_widget_id)
                    .or_default();

                focus_chain.push(FocusNode {
                    widget_id: Some(widget_id),
                    is_focused: false,
                    focus_scope,
                });
            }
            LifeCycle::FocusChanged(value) => {
                self.focus_node.is_focused = *value;

                ctx.submit_command(
                    commands::FOCUS_NODE_FOCUS_CHANGED
                        .with(*value)
                        .to(ctx.widget_id()),
                );
            }
            _ => (),
        }
    }

    /// Handle a layout.
    pub fn layout(&mut self, ctx: &mut LayoutCtx) {
        // Avoid warning
        if self.auto_focus && !self.requested_focus {
            ctx.submit_command(commands::REQUEST_FOCUS.with(ctx.widget_id()));
            self.requested_focus = true;
        }
    }
}

impl Default for FocusController {
    fn default() -> Self {
        Self::new()
    }
}

/// A widget that allow focus to be given to this widget and its descendants.
pub struct Focus<T> {
    child: WidgetPod<T, Box<dyn Widget<T>>>,
    focus_controller: FocusController,
}

impl<T: Data> Focus<T> {
    /// Create a new Focus widget with a child
    pub fn new(child: impl Widget<T> + 'static) -> Self {
        Focus {
            child: WidgetPod::new(child).boxed(),
            focus_controller: FocusController::new(),
        }
    }

    /// Builder-style method to set the `Focus`'s auto focus.
    /// Has focus when the widget is created.
    /// When multiple widgets are auto-focused, the last created widget will gain the focus.
    pub fn with_auto_focus(mut self, auto_focus: bool) -> Self {
        self.focus_controller.auto_focus = auto_focus;
        self
    }
}

impl<T: Data> Widget<T> for Focus<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        ctx.with_focus_node(self.focus_controller.focus_node(), |ctx| {
            if let Event::MouseDown(_) = event {
                ctx.request_focus();
            }

            self.child.event(ctx, event, data, env);
            self.focus_controller.event(ctx, event);
        });
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        ctx.with_focus_node(self.focus_controller.focus_node(), |ctx| {
            self.focus_controller.lifecycle(ctx, event);
            self.child.lifecycle(ctx, event, data, env);
        });
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        ctx.with_focus_node(self.focus_controller.focus_node(), |ctx| {
            self.child.update(ctx, data, env);
        });
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        ctx.with_focus_node(self.focus_controller.focus_node(), |ctx| {
            let size = self.child.layout(ctx, &bc, data, env);
            let rect = size.to_rect();

            self.child.set_layout_rect(ctx, data, env, rect);
            self.focus_controller.layout(ctx);

            size
        })
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_focus_node(self.focus_controller.focus_node(), |ctx| {
            self.child.paint(ctx, data, env);
        });
    }
}
