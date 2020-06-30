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

//! Tests related to layout.

use float_cmp::approx_eq;

use super::*;

#[test]
fn simple_layout() {
    const BOX_WIDTH: f64 = 200.;
    const PADDING: f64 = 10.;

    let id_1 = WidgetId::next();

    let widget = Split::columns(Label::new("hi"), Label::new("there"))
        .fix_size(BOX_WIDTH, BOX_WIDTH)
        .padding(10.0)
        .with_id(id_1)
        .center();

    Harness::create_simple(true, widget, |harness| {
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id_1);
        approx_eq!(
            f64,
            state.layout_rect().x0,
            ((DEFAULT_SIZE.width - BOX_WIDTH) / 2.) - PADDING
        );
    })
}

#[test]
fn row_column() {
    let (id1, id2, id3, id4, id5, id6) = widget_id6();
    let widget = Flex::row()
        .must_fill_main_axis(true)
        .with_flex_child(
            Flex::column()
                .with_flex_child(SizedBox::empty().expand().with_id(id1), 1.0)
                .with_flex_child(SizedBox::empty().expand().with_id(id2), 1.0),
            1.0,
        )
        .with_flex_child(
            Flex::column()
                .with_flex_child(SizedBox::empty().expand().with_id(id3), 1.0)
                .with_flex_child(SizedBox::empty().expand().with_id(id4), 1.0)
                .with_flex_child(SizedBox::empty().expand().with_id(id5), 1.0)
                .with_flex_child(SizedBox::empty().expand().with_id(id6), 1.0)
                .expand_width(),
            1.0,
        );

    Harness::create_simple((), widget, |harness| {
        harness.send_initial_events();
        harness.just_layout();
        let state1 = harness.get_state(id1);
        assert_eq!(state1.layout_rect().origin(), Point::ZERO);
        let state2 = harness.get_state(id2);
        assert_eq!(state2.layout_rect().origin(), Point::new(0., 200.));
        let state3 = harness.get_state(id3);
        assert_eq!(state3.layout_rect().origin(), Point::ZERO);
        let state5 = harness.get_state(id5);
        assert_eq!(state5.layout_rect().origin(), Point::new(0., 200.));
    })
}

#[test]
fn simple_paint_rect() {
    let (id1, id2) = widget_id2();

    let widget = ModularWidget::<(), ()>::new(())
        .layout_fn(|_, ctx, bc, _, _| {
            // this widget paints twenty points above below its layout bounds
            ctx.set_paint_insets(Insets::uniform_xy(0., 20.));
            bc.max()
        })
        .with_id(id1)
        .fix_size(100., 100.)
        .padding(10.0)
        .with_id(id2)
        .background(Color::BLACK)
        .center();

    Harness::create_simple((), widget, |harness| {
        harness.send_initial_events();
        harness.just_layout();

        let state = harness.get_state(id1);

        // offset by padding
        assert_eq!(state.layout_rect().origin(), Point::new(10., 10.,));
        // offset by padding, but then inset by paint insets
        assert_eq!(state.paint_rect().origin(), Point::new(10., -10.,));
        // layout size is fixed
        assert_eq!(state.layout_rect().size(), Size::new(100., 100.,));
        // paint size is modified by insets
        assert_eq!(state.paint_rect().size(), Size::new(100., 140.,));

        // now does the container widget correctly propogate the child's paint rect?
        let state = harness.get_state(id2);

        assert_eq!(state.layout_rect().origin(), Point::ZERO);
        // offset by padding, but then inset by paint insets
        assert_eq!(state.paint_rect().origin(), Point::new(0., -10.,));
        // 100 + 10 on each side
        assert_eq!(state.layout_rect().size(), Size::new(120., 120.,));
        // paint size is modified by insets
        assert_eq!(state.paint_rect().size(), Size::new(120., 140.,));
    })
}

#[test]
/// Does a Flex correctly compute the union of multiple children's paint rects?
fn flex_paint_rect_overflow() {
    let id = WidgetId::next();

    let widget = Flex::row()
        .with_flex_child(
            ModularWidget::new(())
                .layout_fn(|_, ctx, bc, _, _| {
                    ctx.set_paint_insets(Insets::new(20., 0., 0., 0.));
                    bc.constrain(Size::new(10., 10.))
                })
                .expand(),
            1.0,
        )
        .with_flex_child(
            ModularWidget::new(())
                .layout_fn(|_, ctx, bc, _, _| {
                    ctx.set_paint_insets(Insets::new(0., 20., 0., 0.));
                    bc.constrain(Size::new(10., 10.))
                })
                .expand(),
            1.0,
        )
        .with_flex_child(
            ModularWidget::new(())
                .layout_fn(|_, ctx, bc, _, _| {
                    ctx.set_paint_insets(Insets::new(0., 0., 0., 20.));
                    bc.constrain(Size::new(10., 10.))
                })
                .expand(),
            1.0,
        )
        .with_flex_child(
            ModularWidget::new(())
                .layout_fn(|_, ctx, bc, _, _| {
                    ctx.set_paint_insets(Insets::new(0., 0., 20., 0.));
                    bc.constrain(Size::new(10., 10.))
                })
                .expand(),
            1.0,
        )
        .with_id(id)
        .fix_height(200.)
        .padding(10.)
        .center();

    Harness::create_simple((), widget, |harness| {
        harness.set_initial_size(Size::new(300., 300.));
        harness.send_initial_events();
        harness.just_layout();

        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().origin(), Point::new(10., 10.,));
        assert_eq!(state.paint_rect().origin(), Point::new(-10., -10.,));

        // each of our children insets 20. on a different side; their union
        // is a uniform 20. inset.
        let expected_paint_rect = state.layout_rect() + Insets::uniform(20.);
        assert_eq!(state.paint_rect().size(), expected_paint_rect.size());
    })
}
