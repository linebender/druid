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
        assert_eq!(harness.window.focus, Some(left_id));
    })
}
