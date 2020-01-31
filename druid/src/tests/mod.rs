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

use crate::widget::*;
use crate::*;
use harness::*;
use helpers::*;

/// test that the first widget to request focus during an event gets it.
#[test]
fn take_focus() {
    const TAKE_FOCUS: Selector = Selector::new("druid-tests.take-focus");

    /// A widget that takes focus when sent a particular command.
    fn make_focus_taker() -> impl Widget<bool> {
        ModularWidget::new(()).event_fn(|_, ctx, event, _data, _env| {
            if let Event::Command(cmd) = event {
                if cmd.selector == TAKE_FOCUS {
                    ctx.request_focus();
                }
            }
        })
    }

    let left_id = WidgetId::next();
    let left = make_focus_taker().with_id(left_id);
    let right = make_focus_taker();
    let app = Split::vertical(left, right);
    let data = true;

    Harness::create(data, app, |harness| {
        harness.event(Event::Command(TAKE_FOCUS.into()));
        assert_eq!(harness.window().focus, Some(left_id));
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
        harness.layout();
        let state = harness.get_state(id_1).expect("failed to retrieve id_1");
        assert_eq!(
            state.layout_rect.x0,
            ((DEFAULT_SIZE.width - BOX_WIDTH) / 2.) - PADDING
        );
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
