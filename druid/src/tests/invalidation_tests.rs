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

//! Tests related to propagation of invalid rects.

use float_cmp::approx_eq;

use super::*;

#[test]
fn invalidate_union() {
    let id_child1 = WidgetId::next();
    let id_child2 = WidgetId::next();
    let id_parent = WidgetId::next();

    let widget = Split::columns(
        Button::new("hi").with_id(id_child1),
        Button::new("there").with_id(id_child2),
    )
    .with_id(id_parent);

    Harness::create_simple(true, widget, |harness| {
        harness.send_initial_events();
        harness.just_layout();
        let child1_rect = harness.get_state(id_child1).layout_rect();
        let child2_rect = harness.get_state(id_child2).layout_rect();
        harness.event(Event::MouseMove(move_mouse((10., 10.))));
        assert_eq!(harness.window().invalid().rects(), &[child1_rect]);

        // This resets the invalid region.
        harness.paint_invalid();
        assert!(harness.window().invalid().is_empty());

        harness.event(Event::MouseMove(move_mouse((210., 10.))));
        assert_eq!(
            harness.window().invalid().rects(),
            // TODO: this is probably too fragile, because is there any guarantee on the order?
            &[child1_rect, child2_rect]
        );
    });
}

#[test]
fn invalidate_scroll() {
    const RECT: Rect = Rect {
        x0: 30.,
        y0: 40.,
        x1: 40.,
        y1: 50.,
    };

    struct Invalidator {
        invalid: bool,
    }

    impl<T: Data> Widget<T> for Invalidator {
        fn event(&mut self, ctx: &mut EventCtx, _: &Event, _: &mut T, _: &Env) {
            ctx.request_paint_rect(RECT);
        }

        fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &T, _: &Env) {}
        fn update(&mut self, _: &mut UpdateCtx, _: &T, _: &T, _: &Env) {}
        fn layout(&mut self, _: &mut LayoutCtx, _: &BoxConstraints, _: &T, _: &Env) -> Size {
            Size::new(1000., 1000.)
        }

        fn paint(&mut self, ctx: &mut PaintCtx, _: &T, _: &Env) {
            assert_eq!(ctx.region().rects().len(), 1);
            let rect = ctx.region().rects().first().unwrap();

            approx_eq!(f64, rect.x0, 30.);
            approx_eq!(f64, rect.y0, 40.);
            approx_eq!(f64, rect.x1, 40.);
            approx_eq!(f64, rect.y1, 50.);
            self.invalid = false;
        }
    }

    let id = WidgetId::next();
    let scroll_id = WidgetId::next();
    let invalidator = IdentityWrapper::wrap(Invalidator { invalid: false }, id);
    let scroll = Scroll::new(invalidator).with_id(scroll_id);

    Harness::create_simple(true, scroll, |harness| {
        harness.send_initial_events();
        harness.just_layout();

        // Sending an event should cause RECT to get invalidated.
        harness.event(Event::MouseMove(move_mouse((10., 10.))));
        assert_eq!(harness.window().invalid().rects(), &[RECT]);

        // This resets the invalid region, and our widget checks to make sure it sees the right
        // invalid region in the paint function.
        harness.paint_invalid();
        assert!(harness.window().invalid().is_empty());

        harness.event(Event::Wheel(scroll_mouse((10., 10.), (7.0, 9.0))));
        // Scrolling invalidates the whole window.
        assert_eq!(
            harness.window().invalid().rects(),
            &[Size::new(400., 400.).to_rect()]
        );
        harness.window_mut().invalid_mut().clear();

        // After the scroll, the window should see the translated invalid regions...
        harness.event(Event::MouseMove(move_mouse((10., 10.))));
        assert_eq!(
            harness.window().invalid().rects(),
            &[RECT - Vec2::new(7.0, 9.0)]
        );
        // ...but in its paint callback, the widget will see the invalid region relative to itself.
        harness.paint_invalid();
    });
}

// TODO: one with scroll
