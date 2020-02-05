// Copyright 2020 The xi-editor Authors.
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

//! Additional unit tests that cross file or module boundaries.

mod harness;
mod helpers;

use std::cell::Cell;
use std::rc::Rc;

use crate::widget::*;
use crate::*;
use harness::*;
use helpers::*;

#[test]
fn take_focus() {
    const TAKE_FOCUS: Selector = Selector::new("druid-tests.take-focus");

    /// A widget that takes focus when sent a particular command.
    /// The widget records focus change events into the inner cell.
    fn make_focus_taker(inner: Rc<Cell<Option<bool>>>) -> impl Widget<bool> {
        ModularWidget::new(inner)
            .event_fn(|_, ctx, event, _data, _env| {
                if let Event::Command(cmd) = event {
                    if cmd.selector == TAKE_FOCUS {
                        ctx.request_focus();
                    }
                }
            })
            .lifecycle_fn(|is_focused, _, event, _data, _env| {
                if let LifeCycle::FocusChanged(focus) = event {
                    is_focused.set(Some(*focus));
                }
            })
    }

    let (id_1, id_2, _id_3) = (WidgetId::next(), WidgetId::next(), WidgetId::next());

    // we use these so that we can check the widget's internal state
    let left_focus: Rc<Cell<Option<bool>>> = Default::default();
    let right_focus: Rc<Cell<Option<bool>>> = Default::default();
    assert!(left_focus.get().is_none());

    let left = make_focus_taker(left_focus.clone()).with_id(id_1);
    let right = make_focus_taker(right_focus.clone()).with_id(id_2);
    let app = Split::vertical(left, right).padding(5.0);
    let data = true;

    Harness::create(data, app, |harness| {
        harness.send_initial_events();
        // nobody should have focus
        assert!(left_focus.get().is_none());
        assert!(right_focus.get().is_none());

        // this is sent to all widgets; the first widget to request focus should get it
        harness.submit_command(TAKE_FOCUS, None);
        assert_eq!(harness.window().focus, Some(id_1));
        assert_eq!(left_focus.get(), Some(true));
        assert_eq!(right_focus.get(), None);

        // this is sent to a specific widget; it should get focus
        harness.submit_command(TAKE_FOCUS, id_2);
        assert_eq!(harness.window().focus, Some(id_2));
        assert_eq!(left_focus.get(), Some(false));
        assert_eq!(right_focus.get(), Some(true));
    })
}

#[test]
fn simple_layout() {
    const BOX_WIDTH: f64 = 200.;
    const PADDING: f64 = 10.;

    let id_1 = WidgetId::next();

    let widget = Split::vertical(Label::new("hi"), Label::new("there"))
        .fix_size(BOX_WIDTH, BOX_WIDTH)
        .padding(10.0)
        .with_id(id_1)
        .center();

    Harness::create(true, widget, |harness| {
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id_1).expect("failed to retrieve id_1");
        assert_eq!(
            state.layout_rect.x0,
            ((DEFAULT_SIZE.width - BOX_WIDTH) / 2.) - PADDING
        );
    })
}

#[test]
fn participate_in_autofocus() {
    let (id_1, id_2, id_3) = (WidgetId::next(), WidgetId::next(), WidgetId::next());
    let (id_4, id_5, id_6) = (WidgetId::next(), WidgetId::next(), WidgetId::next());

    // this widget starts with a single child, and will replace them with a split
    // when we send it a command.
    let replacer = ReplaceChild::new(TextBox::raw().with_id(id_4), move || {
        Split::vertical(TextBox::raw().with_id(id_5), TextBox::raw().with_id(id_6))
    });

    let widget = Split::vertical(
        Flex::row()
            .with_child(TextBox::raw().with_id(id_1), 1.0)
            .with_child(TextBox::raw().with_id(id_2), 1.0)
            .with_child(TextBox::raw().with_id(id_3), 1.0),
        replacer,
    );

    Harness::create("my test text".to_string(), widget, |harness| {
        // verify that all widgets are marked as having children_changed
        // (this should always be true for a new widget)
        harness.inspect_state(|state| assert!(state.children_changed));

        harness.send_initial_events();
        // verify that we start out with four widgets registered for focus
        assert_eq!(harness.window().focus_chain(), &[id_1, id_2, id_3, id_4]);

        // tell the replacer widget to swap its children
        harness.submit_command(REPLACE_CHILD, None);

        // verify that the two new children are registered for focus.
        assert_eq!(
            harness.window().focus_chain(),
            &[id_1, id_2, id_3, id_5, id_6]
        );

        // verify that no widgets still report that their children changed:
        harness.inspect_state(|state| assert!(!state.children_changed))
    })
}

#[test]
fn child_tracking() {
    let (id_1, id_2, id_3) = (WidgetId::next(), WidgetId::next(), WidgetId::next());
    let id_4 = WidgetId::next();

    let widget = Split::vertical(
        SizedBox::empty().with_id(id_1),
        SizedBox::empty().with_id(id_2),
    )
    .with_id(id_3)
    .padding(5.0)
    .with_id(id_4);

    Harness::create(true, widget, |harness| {
        harness.send_initial_events();
        let root = harness.get_state(id_4).unwrap();
        assert!(root.children.contains(&id_1));
        assert!(root.children.contains(&id_2));
        assert!(root.children.contains(&id_3));
        assert_eq!(root.children.entry_count(), 3);

        let split = harness.get_state(id_3).unwrap();
        assert!(split.children.contains(&id_1));
        assert!(split.children.contains(&id_2));
        assert_eq!(split.children.entry_count(), 2);
    });
}
