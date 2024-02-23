// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Tests related to layout.

use float_cmp::assert_approx_eq;
use test_log::test;

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
        assert_approx_eq!(
            f64,
            state.layout_rect().x0,
            ((DEFAULT_SIZE.width - BOX_WIDTH) / 2.) - PADDING
        );
    })
}

#[test]
fn row_column() {
    let [id1, id2, id3, id4, id5, id6] = widget_ids();
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
    let [id1, id2] = widget_ids();

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

        // now does the container widget correctly propagate the child's paint rect?
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

use crate::tests::harness::*;
use crate::widget::AspectRatioBox;
use crate::widget::Label;
use crate::WidgetExt;

#[test]
fn aspect_ratio_tight_constraints() {
    let id = WidgetId::next();
    let (width, height) = (400., 400.);
    let aspect = AspectRatioBox::<()>::new(Label::new("hello!"), 1.0)
        .with_id(id)
        .fix_width(width)
        .fix_height(height)
        .center();

    let (window_width, window_height) = (600., 600.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(width, height));
    });
}

#[test]
fn aspect_ratio_infinite_constraints() {
    let id = WidgetId::next();
    let (width, height) = (100., 100.);
    let label = Label::new("hello!").fix_width(width).height(height);
    let aspect = AspectRatioBox::<()>::new(label, 1.0)
        .with_id(id)
        .scroll()
        .center();

    let (window_width, window_height) = (600., 600.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(width, height));
    });
}

#[test]
fn aspect_ratio_tight_constraint_on_width() {
    let id = WidgetId::next();
    let label = Label::new("hello!");
    let aspect = AspectRatioBox::<()>::new(label, 2.0)
        .with_id(id)
        .fix_width(300.)
        .center();

    let (window_width, window_height) = (600., 50.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(300., 50.));
    });
}

#[test]
fn aspect_ratio() {
    let id = WidgetId::next();
    let label = Label::new("hello!");
    let aspect = AspectRatioBox::<()>::new(label, 2.0)
        .with_id(id)
        .center()
        .center();

    let (window_width, window_height) = (1000., 1000.);

    Harness::create_simple((), aspect, |harness| {
        harness.set_initial_size(Size::new(window_width, window_height));
        harness.send_initial_events();
        harness.just_layout();
        let state = harness.get_state(id);
        assert_eq!(state.layout_rect().size(), Size::new(1000., 500.));
    });
}
