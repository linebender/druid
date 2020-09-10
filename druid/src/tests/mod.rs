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

//! Additional unit tests that cross file or module boundaries.

pub(crate) mod harness;
mod helpers;
mod layout_tests;

use std::cell::Cell;
use std::env;
use std::fs;
use std::rc::Rc;

use crate::widget::*;
use crate::*;
use harness::*;
use helpers::*;
use kurbo::Vec2;

/// This function creates a temporary directory and returns a PathBuf to it.
///
/// This directory will be created relative to the executable and will therefor
/// be created in the target directory for tests when running with cargo. The
/// directory will be cleaned up at the end of the PathBufs lifetime. This
/// uses the `tempfile` crate.
#[allow(dead_code)]
pub fn temp_dir_for_test() -> std::path::PathBuf {
    let current_exe_path = env::current_exe().unwrap();
    let mut exe_dir = current_exe_path.parent().unwrap();
    if exe_dir.ends_with("deps") {
        exe_dir = exe_dir.parent().unwrap();
    }
    let test_dir = exe_dir.parent().unwrap().join("tests");
    fs::create_dir_all(&test_dir).unwrap();
    tempfile::Builder::new()
        .prefix("TempDir")
        .tempdir_in(test_dir)
        .unwrap()
        .into_path()
}

/// test that the first widget to request focus during an event gets it.
#[test]
fn propogate_hot() {
    let (button, pad, root, empty) = widget_id4();

    let root_rec = Recording::default();
    let padding_rec = Recording::default();
    let button_rec = Recording::default();

    let widget = Split::columns(
        SizedBox::empty().with_id(empty),
        Button::new("hot")
            .record(&button_rec)
            .with_id(button)
            .padding(50.)
            .record(&padding_rec)
            .with_id(pad),
    )
    .record(&root_rec)
    .with_id(root);

    fn move_mouse(x: f64, y: f64) -> MouseEvent {
        let pos = Point::new(x, y);
        MouseEvent {
            pos,
            window_pos: pos,
            buttons: MouseButtons::default(),
            mods: Modifiers::default(),
            count: 0,
            focus: false,
            button: MouseButton::None,
            wheel_delta: Vec2::ZERO,
        }
    }
    #[allow(clippy::cognitive_complexity)]
    Harness::create_simple((), widget, |harness| {
        harness.send_initial_events();
        harness.just_layout();

        // we don't care about setup events, so discard them now.
        root_rec.clear();
        padding_rec.clear();
        button_rec.clear();

        harness.inspect_state(|state| assert!(!state.is_hot));

        // What we are doing here is moving the mouse to different widgets,
        // and verifying both the widget's `is_hot` status and also that
        // each widget received the expected HotChanged messages.

        harness.event(Event::MouseMove(move_mouse(10., 10.)));
        assert!(harness.get_state(root).is_hot);
        assert!(harness.get_state(empty).is_hot);
        assert!(!harness.get_state(pad).is_hot);

        assert!(matches!(
            root_rec.next(),
            Record::L(LifeCycle::HotChanged(true))
        ));
        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());

        harness.event(Event::MouseMove(move_mouse(210., 10.)));

        assert!(harness.get_state(root).is_hot);
        assert!(!harness.get_state(empty).is_hot);
        assert!(!harness.get_state(button).is_hot);
        assert!(harness.get_state(pad).is_hot);

        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            padding_rec.next(),
            Record::L(LifeCycle::HotChanged(true))
        ));
        assert!(matches!(padding_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());

        harness.event(Event::MouseMove(move_mouse(260., 60.)));
        assert!(harness.get_state(root).is_hot);
        assert!(!harness.get_state(empty).is_hot);
        assert!(harness.get_state(button).is_hot);
        assert!(harness.get_state(pad).is_hot);

        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(padding_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            button_rec.next(),
            Record::L(LifeCycle::HotChanged(true))
        ));
        assert!(matches!(button_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());

        harness.event(Event::MouseMove(move_mouse(10., 10.)));
        assert!(harness.get_state(root).is_hot);
        assert!(harness.get_state(empty).is_hot);
        assert!(!harness.get_state(button).is_hot);
        assert!(!harness.get_state(pad).is_hot);

        assert!(matches!(root_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            padding_rec.next(),
            Record::L(LifeCycle::HotChanged(false))
        ));
        assert!(matches!(padding_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(matches!(
            button_rec.next(),
            Record::L(LifeCycle::HotChanged(false))
        ));
        assert!(matches!(button_rec.next(), Record::E(Event::MouseMove(_))));
        assert!(root_rec.is_empty() && padding_rec.is_empty() && button_rec.is_empty());
    });
}
#[test]
fn take_focus() {
    const TAKE_FOCUS: Selector = Selector::new("druid-tests.take-focus");

    /// A widget that takes focus when sent a particular command.
    /// The widget records focus change events into the inner cell.
    fn make_focus_taker(inner: Rc<Cell<Option<bool>>>) -> impl Widget<bool> {
        ModularWidget::new(inner)
            .event_fn(|_, ctx, event, _data, _env| {
                if let Event::Command(cmd) = event {
                    if cmd.is(TAKE_FOCUS) {
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

    let (id_1, id_2, _id_3) = widget_id3();

    // we use these so that we can check the widget's internal state
    let left_focus: Rc<Cell<Option<bool>>> = Default::default();
    let right_focus: Rc<Cell<Option<bool>>> = Default::default();
    assert!(left_focus.get().is_none());

    let left = make_focus_taker(left_focus.clone()).with_id(id_1);
    let right = make_focus_taker(right_focus.clone()).with_id(id_2);
    let app = Split::columns(left, right).padding(5.0);
    let data = true;

    Harness::create_simple(data, app, |harness| {
        harness.send_initial_events();
        // nobody should have focus
        assert!(left_focus.get().is_none());
        assert!(right_focus.get().is_none());

        // this is sent to all widgets; the last widget to request focus should get it
        harness.submit_command(TAKE_FOCUS);
        assert_eq!(harness.window().focus, Some(id_2));
        assert_eq!(left_focus.get(), None);
        assert_eq!(right_focus.get(), Some(true));

        // this is sent to all widgets; the last widget to request focus should still get it
        // NOTE: This tests siblings in particular, so careful when moving away from Split.
        harness.submit_command(TAKE_FOCUS);
        assert_eq!(harness.window().focus, Some(id_2));
        assert_eq!(left_focus.get(), None);
        assert_eq!(right_focus.get(), Some(true));

        // this is sent to a specific widget; it should get focus
        harness.submit_command(TAKE_FOCUS.to(id_1));
        assert_eq!(harness.window().focus, Some(id_1));
        assert_eq!(left_focus.get(), Some(true));
        assert_eq!(right_focus.get(), Some(false));

        // this is sent to a specific widget; it should get focus
        harness.submit_command(TAKE_FOCUS.to(id_2));
        assert_eq!(harness.window().focus, Some(id_2));
        assert_eq!(left_focus.get(), Some(false));
        assert_eq!(right_focus.get(), Some(true));
    })
}

#[test]
fn focus_changed() {
    const TAKE_FOCUS: Selector = Selector::new("druid-tests.take-focus");
    const ALL_TAKE_FOCUS_BEFORE: Selector = Selector::new("druid-tests.take-focus-before");
    const ALL_TAKE_FOCUS_AFTER: Selector = Selector::new("druid-tests.take-focus-after");

    fn make_focus_container(children: Vec<WidgetPod<(), Box<dyn Widget<()>>>>) -> impl Widget<()> {
        ModularWidget::new(children)
            .event_fn(|children, ctx, event, data, env| {
                if let Event::Command(cmd) = event {
                    if cmd.is(TAKE_FOCUS) {
                        ctx.request_focus();
                        // Stop propagating this command so children
                        // aren't requesting focus too.
                        ctx.set_handled();
                    } else if cmd.is(ALL_TAKE_FOCUS_BEFORE) {
                        ctx.request_focus();
                    }
                }
                children
                    .iter_mut()
                    .for_each(|a| a.event(ctx, event, data, env));
                if let Event::Command(cmd) = event {
                    if cmd.is(ALL_TAKE_FOCUS_AFTER) {
                        ctx.request_focus();
                    }
                }
            })
            .lifecycle_fn(|children, ctx, event, data, env| {
                children
                    .iter_mut()
                    .for_each(|a| a.lifecycle(ctx, event, data, env));
            })
    }

    let a_rec = Recording::default();
    let b_rec = Recording::default();
    let c_rec = Recording::default();

    let (id_a, id_b, id_c) = widget_id3();

    // a contains b which contains c
    let c = make_focus_container(vec![]).record(&c_rec).with_id(id_c);
    let b = make_focus_container(vec![WidgetPod::new(c).boxed()])
        .record(&b_rec)
        .with_id(id_b);
    let a = make_focus_container(vec![WidgetPod::new(b).boxed()])
        .record(&a_rec)
        .with_id(id_a);

    let f = |a| match a {
        Record::L(LifeCycle::FocusChanged(c)) => Some(c),
        _ => None,
    };
    let no_change = |a: &Recording| a.drain().filter_map(f).count() == 0;
    let changed = |a: &Recording, b| a.drain().filter_map(f).eq(std::iter::once(b));

    Harness::create_simple((), a, |harness| {
        harness.send_initial_events();

        // focus none -> a
        harness.submit_command(TAKE_FOCUS.to(id_a));
        assert_eq!(harness.window().focus, Some(id_a));
        assert!(changed(&a_rec, true));
        assert!(no_change(&b_rec));
        assert!(no_change(&c_rec));

        // focus a -> b
        harness.submit_command(TAKE_FOCUS.to(id_b));
        assert_eq!(harness.window().focus, Some(id_b));
        assert!(changed(&a_rec, false));
        assert!(changed(&b_rec, true));
        assert!(no_change(&c_rec));

        // focus b -> c
        harness.submit_command(TAKE_FOCUS.to(id_c));
        assert_eq!(harness.window().focus, Some(id_c));
        assert!(no_change(&a_rec));
        assert!(changed(&b_rec, false));
        assert!(changed(&c_rec, true));

        // focus c -> a
        harness.submit_command(TAKE_FOCUS.to(id_a));
        assert_eq!(harness.window().focus, Some(id_a));
        assert!(changed(&a_rec, true));
        assert!(no_change(&b_rec));
        assert!(changed(&c_rec, false));

        // all focus before passing down the event
        harness.submit_command(ALL_TAKE_FOCUS_BEFORE);
        assert_eq!(harness.window().focus, Some(id_c));
        assert!(changed(&a_rec, false));
        assert!(no_change(&b_rec));
        assert!(changed(&c_rec, true));

        // all focus after passing down the event
        harness.submit_command(ALL_TAKE_FOCUS_AFTER);
        assert_eq!(harness.window().focus, Some(id_a));
        assert!(changed(&a_rec, true));
        assert!(no_change(&b_rec));
        assert!(changed(&c_rec, false));
    })
}

#[test]
fn simple_lifecyle() {
    let record = Recording::default();
    let widget = SizedBox::empty().record(&record);
    Harness::create_simple(true, widget, |harness| {
        harness.send_initial_events();
        assert!(matches!(record.next(), Record::L(LifeCycle::WidgetAdded)));
        assert!(matches!(record.next(), Record::E(Event::WindowConnected)));
        assert!(matches!(record.next(), Record::E(Event::WindowSize(_))));
        assert!(record.is_empty());
    })
}

#[test]
/// Test that lifecycle events are sent correctly to a child added during event
/// handling
fn adding_child_lifecycle() {
    let record = Recording::default();
    let record_new_child = Recording::default();
    let record_new_child2 = record_new_child.clone();

    let replacer = ReplaceChild::new(TextBox::new(), move || {
        Split::columns(TextBox::new(), TextBox::new().record(&record_new_child2))
    });

    let widget = Split::columns(Label::new("hi").record(&record), replacer);

    Harness::create_simple(String::new(), widget, |harness| {
        harness.send_initial_events();

        assert!(matches!(record.next(), Record::L(LifeCycle::WidgetAdded)));
        assert!(matches!(record.next(), Record::E(Event::WindowConnected)));
        assert!(record.is_empty());

        assert!(record_new_child.is_empty());

        harness.submit_command(REPLACE_CHILD);

        assert!(matches!(record.next(), Record::E(Event::Command(_))));

        assert!(matches!(
            record_new_child.next(),
            Record::L(LifeCycle::WidgetAdded)
        ));
        assert!(record_new_child.is_empty());
    })
}

#[test]
fn participate_in_autofocus() {
    let (id_1, id_2, id_3, id_4, id_5, id_6) = widget_id6();

    // this widget starts with a single child, and will replace them with a split
    // when we send it a command.
    let replacer = ReplaceChild::new(TextBox::new().with_id(id_4), move || {
        Split::columns(TextBox::new().with_id(id_5), TextBox::new().with_id(id_6))
    });

    let widget = Split::columns(
        Flex::row()
            .with_flex_child(TextBox::new().with_id(id_1), 1.0)
            .with_flex_child(TextBox::new().with_id(id_2), 1.0)
            .with_flex_child(TextBox::new().with_id(id_3), 1.0),
        replacer,
    );

    Harness::create_simple("my test text".to_string(), widget, |harness| {
        // verify that all widgets are marked as having children_changed
        // (this should always be true for a new widget)
        harness.inspect_state(|state| assert!(state.children_changed));

        harness.send_initial_events();
        // verify that we start out with four widgets registered for focus
        assert_eq!(harness.window().focus_chain(), &[id_1, id_2, id_3, id_4]);

        // tell the replacer widget to swap its children
        harness.submit_command(REPLACE_CHILD);

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
    let (id_1, id_2, id_3, id_4) = widget_id4();

    let widget = Split::columns(
        SizedBox::empty().with_id(id_1),
        SizedBox::empty().with_id(id_2),
    )
    .with_id(id_3)
    .padding(5.0)
    .with_id(id_4);

    Harness::create_simple(true, widget, |harness| {
        harness.send_initial_events();
        let root = harness.get_state(id_4);
        assert_eq!(root.children.entry_count(), 3);
        assert!(root.children.may_contain(&id_1));
        assert!(root.children.may_contain(&id_2));
        assert!(root.children.may_contain(&id_3));

        let split = harness.get_state(id_3);
        assert!(split.children.may_contain(&id_1));
        assert!(split.children.may_contain(&id_2));
        assert_eq!(split.children.entry_count(), 2);
    });
}

#[test]
/// Test that all children are registered correctly after a child is replaced.
fn register_after_adding_child() {
    let (id_1, id_2, id_3, id_4, id_5, id_6) = widget_id6();
    let id_7 = WidgetId::next();

    let replacer = ReplaceChild::new(TextBox::new().with_id(id_1), move || {
        Split::columns(TextBox::new().with_id(id_2), TextBox::new().with_id(id_3)).with_id(id_7)
    })
    .with_id(id_6);

    let widget = Split::columns(Label::new("hi").with_id(id_4), replacer).with_id(id_5);

    Harness::create_simple(String::new(), widget, |harness| {
        harness.send_initial_events();

        assert!(harness.get_state(id_5).children.may_contain(&id_6));
        assert!(harness.get_state(id_5).children.may_contain(&id_1));
        assert!(harness.get_state(id_5).children.may_contain(&id_4));
        assert_eq!(harness.get_state(id_5).children.entry_count(), 3);

        harness.submit_command(REPLACE_CHILD);

        assert!(harness.get_state(id_5).children.may_contain(&id_6));
        assert!(harness.get_state(id_5).children.may_contain(&id_4));
        assert!(harness.get_state(id_5).children.may_contain(&id_7));
        assert!(harness.get_state(id_5).children.may_contain(&id_2));
        assert!(harness.get_state(id_5).children.may_contain(&id_3));
        assert_eq!(harness.get_state(id_5).children.entry_count(), 5);
    })
}

#[test]
/// Test that request_update actually causes the request.
fn request_update() {
    const REQUEST_UPDATE: Selector = Selector::new("druid-tests.request_update");
    let updated: Rc<Cell<bool>> = Default::default();
    let updated_clone = updated.clone();

    let widget = ModularWidget::new(())
        .event_fn(|_, ctx, event, _data, _env| {
            if matches!(event, Event::Command(cmd) if cmd.is(REQUEST_UPDATE)) {
                ctx.request_update();
            }
        })
        .update_fn(move |_, _ctx, _old_data, _data, _env| {
            updated_clone.set(true);
        });
    Harness::create_simple((), widget, |harness| {
        harness.send_initial_events();
        assert!(!updated.get());
        harness.submit_command(REQUEST_UPDATE);
        assert!(updated.get());
    })
}
