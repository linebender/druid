// Copyright 2018 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Common widgets.

// First as it defines macros
#[macro_use]
mod widget_wrapper;

mod added;
mod align;
mod aspect_ratio_box;
mod button;
mod checkbox;
mod click;
mod clip_box;
mod common;
mod container;
mod controller;
mod disable_if;
mod either;
mod env_scope;
mod flex;
mod identity_wrapper;
mod image;
mod intrinsic_width;
mod invalidation;
mod label;
mod lens_wrap;
mod list;
mod maybe;
mod padding;
mod painter;
mod parse;
mod progress_bar;
mod radio;
mod scope;
mod scroll;
mod sized_box;
mod slider;
mod spinner;
mod split;
mod stepper;
#[cfg(feature = "svg")]
#[cfg_attr(docsrs, doc(cfg(feature = "svg")))]
mod svg;
mod switch;
mod tabs;
mod textbox;
mod value_textbox;
mod view_switcher;
#[allow(clippy::module_inception)]
mod widget;
mod widget_ext;
mod z_stack;

pub use self::image::Image;
pub use added::Added;
pub use align::Align;
pub use aspect_ratio_box::AspectRatioBox;
pub use button::Button;
pub use checkbox::Checkbox;
pub use click::Click;
pub use clip_box::{ClipBox, Viewport};
pub use common::FillStrat;
pub use container::Container;
pub use controller::{Controller, ControllerHost};
pub use disable_if::DisabledIf;
pub use either::Either;
pub use env_scope::EnvScope;
pub use flex::{Axis, CrossAxisAlignment, Flex, FlexParams, MainAxisAlignment};
pub use identity_wrapper::IdentityWrapper;
pub use intrinsic_width::IntrinsicWidth;
pub use label::{Label, LabelText, LineBreaking, RawLabel};
pub use lens_wrap::LensWrap;
pub use list::{List, ListIter};
pub use maybe::Maybe;
pub use padding::Padding;
pub use painter::{BackgroundBrush, Painter};
#[allow(deprecated)]
pub use parse::Parse;
pub use progress_bar::ProgressBar;
pub use radio::{Radio, RadioGroup};
pub use scope::{DefaultScopePolicy, LensScopeTransfer, Scope, ScopePolicy, ScopeTransfer};
pub use scroll::Scroll;
pub use sized_box::SizedBox;
pub use slider::{KnobStyle, RangeSlider, Slider};
pub use spinner::Spinner;
pub use split::Split;
pub use stepper::Stepper;
#[cfg(feature = "svg")]
pub use svg::{Svg, SvgData};
pub use switch::Switch;
pub use tabs::{AddTab, TabInfo, Tabs, TabsEdge, TabsPolicy, TabsState, TabsTransition};
pub use textbox::TextBox;
pub use value_textbox::{TextBoxEvent, ValidationDelegate, ValueTextBox};
pub use view_switcher::ViewSwitcher;
pub use widget::{Widget, WidgetId};
pub use widget_ext::WidgetExt;
pub use widget_wrapper::WidgetWrapper;
pub use z_stack::ZStack;

/// The types required to implement a [`Widget`].
pub mod prelude {
    // Wildcard because rustdoc has trouble inlining docs of two things called Data
    pub use crate::data::*;

    #[doc(inline)]
    pub use crate::{
        BoxConstraints, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
        RenderContext, Size, UpdateCtx, Widget, WidgetId,
    };
}
