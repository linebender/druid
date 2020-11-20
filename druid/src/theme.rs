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

//! Theme keys and initial values.

#![allow(missing_docs)]

use crate::piet::Color;

use crate::{Env, FontDescriptor, FontFamily, FontStyle, FontWeight, Insets, Key};

pub const WINDOW_BACKGROUND_COLOR: Key<Color> =
    Key::new("org.linebender.druid.theme.window_background_color");

pub const LABEL_COLOR: Key<Color> = Key::new("org.linebender.druid.theme.label_color");
pub const PLACEHOLDER_COLOR: Key<Color> = Key::new("org.linebender.druid.theme.placeholder_color");

pub const PRIMARY_LIGHT: Key<Color> = Key::new("org.linebender.druid.theme.primary_light");
pub const PRIMARY_DARK: Key<Color> = Key::new("org.linebender.druid.theme.primary_dark");
pub const PROGRESS_BAR_RADIUS: Key<f64> =
    Key::new("org.linebender.druid.theme.progress_bar_radius");
pub const BACKGROUND_LIGHT: Key<Color> = Key::new("org.linebender.druid.theme.background_light");
pub const BACKGROUND_DARK: Key<Color> = Key::new("org.linebender.druid.theme.background_dark");
pub const FOREGROUND_LIGHT: Key<Color> = Key::new("org.linebender.druid.theme.foreground_light");
pub const FOREGROUND_DARK: Key<Color> = Key::new("org.linebender.druid.theme.foreground_dark");
pub const BUTTON_DARK: Key<Color> = Key::new("org.linebender.druid.theme.button_dark");
pub const BUTTON_LIGHT: Key<Color> = Key::new("org.linebender.druid.theme.button_light");
pub const BUTTON_BORDER_RADIUS: Key<f64> = Key::new("org.linebender.druid.theme.button_radius");
pub const BUTTON_BORDER_WIDTH: Key<f64> =
    Key::new("org.linebender.druid.theme.button_border_width");
pub const BORDER_DARK: Key<Color> = Key::new("org.linebender.druid.theme.border_dark");
pub const BORDER_LIGHT: Key<Color> = Key::new("org.linebender.druid.theme.border_light");
pub const SELECTION_COLOR: Key<Color> = Key::new("org.linebender.druid.theme.selection_color");
pub const SELECTION_TEXT_COLOR: Key<Color> =
    Key::new("org.linebender.druid.theme.selection_text_color");
pub const CURSOR_COLOR: Key<Color> = Key::new("org.linebender.druid.theme.cursor_color");

pub const TEXT_SIZE_NORMAL: Key<f64> = Key::new("org.linebender.druid.theme.text_size_normal");
pub const TEXT_SIZE_LARGE: Key<f64> = Key::new("org.linebender.druid.theme.text_size_large");
pub const BASIC_WIDGET_HEIGHT: Key<f64> =
    Key::new("org.linebender.druid.theme.basic_widget_height");

/// The default font for labels, buttons, text boxes, and other UI elements.
pub const UI_FONT: Key<FontDescriptor> = Key::new("org.linebender.druid.theme.ui-font");

/// A bold version of the default UI font.
pub const UI_FONT_BOLD: Key<FontDescriptor> = Key::new("org.linebender.druid.theme.ui-font-bold");

/// An Italic version of the default UI font.
pub const UI_FONT_ITALIC: Key<FontDescriptor> =
    Key::new("org.linebender.druid.theme.ui-font-italic");

/// The default minimum width for a 'wide' widget; a textbox, slider, progress bar, etc.
pub const WIDE_WIDGET_WIDTH: Key<f64> = Key::new("org.linebender.druid.theme.long-widget-width");
pub const BORDERED_WIDGET_HEIGHT: Key<f64> =
    Key::new("org.linebender.druid.theme.bordered_widget_height");

pub const TEXTBOX_BORDER_RADIUS: Key<f64> =
    Key::new("org.linebender.druid.theme.textbox_border_radius");
pub const TEXTBOX_BORDER_WIDTH: Key<f64> =
    Key::new("org.linebender.druid.theme.textbox_border_width");
pub const TEXTBOX_INSETS: Key<Insets> = Key::new("org.linebender.druid.theme.textbox_insets");

/// The default horizontal spacing between widgets.
pub const WIDGET_PADDING_HORIZONTAL: Key<f64> =
    Key::new("org.linebender.druid.theme.widget-padding-h");
/// The default vertical spacing between widgets.
pub const WIDGET_PADDING_VERTICAL: Key<f64> =
    Key::new("org.linebender.druid.theme.widget-padding-v");
/// The default internal (horizontal) padding for visually distinct components
/// of a widget; for instance between a checkbox and its label.
pub const WIDGET_CONTROL_COMPONENT_PADDING: Key<f64> =
    Key::new("org.linebender.druid.theme.widget-padding-control-label");

pub const SCROLLBAR_COLOR: Key<Color> = Key::new("org.linebender.druid.theme.scrollbar_color");
pub const SCROLLBAR_BORDER_COLOR: Key<Color> =
    Key::new("org.linebender.druid.theme.scrollbar_border_color");
pub const SCROLLBAR_MAX_OPACITY: Key<f64> =
    Key::new("org.linebender.druid.theme.scrollbar_max_opacity");
pub const SCROLLBAR_FADE_DELAY: Key<u64> =
    Key::new("org.linebender.druid.theme.scrollbar_fade_time");
pub const SCROLLBAR_WIDTH: Key<f64> = Key::new("org.linebender.druid.theme.scrollbar_width");
pub const SCROLLBAR_PAD: Key<f64> = Key::new("org.linebender.druid.theme.scrollbar_pad");
pub const SCROLLBAR_RADIUS: Key<f64> = Key::new("org.linebender.druid.theme.scrollbar_radius");
pub const SCROLLBAR_EDGE_WIDTH: Key<f64> =
    Key::new("org.linebender.druid.theme.scrollbar_edge_width");

/// An initial theme.
pub(crate) fn add_to_env(env: Env) -> Env {
    env.adding(WINDOW_BACKGROUND_COLOR, Color::rgb8(0x29, 0x29, 0x29))
        .adding(LABEL_COLOR, Color::rgb8(0xf0, 0xf0, 0xea))
        .adding(PLACEHOLDER_COLOR, Color::rgb8(0x80, 0x80, 0x80))
        .adding(PRIMARY_LIGHT, Color::rgb8(0x5c, 0xc4, 0xff))
        .adding(PRIMARY_DARK, Color::rgb8(0x00, 0x8d, 0xdd))
        .adding(PROGRESS_BAR_RADIUS, 4.)
        .adding(BACKGROUND_LIGHT, Color::rgb8(0x3a, 0x3a, 0x3a))
        .adding(BACKGROUND_DARK, Color::rgb8(0x31, 0x31, 0x31))
        .adding(FOREGROUND_LIGHT, Color::rgb8(0xf9, 0xf9, 0xf9))
        .adding(FOREGROUND_DARK, Color::rgb8(0xbf, 0xbf, 0xbf))
        .adding(BUTTON_DARK, Color::BLACK)
        .adding(BUTTON_LIGHT, Color::rgb8(0x21, 0x21, 0x21))
        .adding(BUTTON_BORDER_RADIUS, 4.)
        .adding(BUTTON_BORDER_WIDTH, 2.)
        .adding(BORDER_DARK, Color::rgb8(0x3a, 0x3a, 0x3a))
        .adding(BORDER_LIGHT, Color::rgb8(0xa1, 0xa1, 0xa1))
        .adding(SELECTION_COLOR, Color::rgb8(0xf3, 0x00, 0x21))
        .adding(SELECTION_TEXT_COLOR, Color::rgb8(0x00, 0x00, 0x00))
        .adding(CURSOR_COLOR, Color::WHITE)
        .adding(TEXT_SIZE_NORMAL, 15.0)
        .adding(TEXT_SIZE_LARGE, 24.0)
        .adding(BASIC_WIDGET_HEIGHT, 18.0)
        .adding(WIDE_WIDGET_WIDTH, 100.)
        .adding(BORDERED_WIDGET_HEIGHT, 24.0)
        .adding(TEXTBOX_BORDER_RADIUS, 2.)
        .adding(TEXTBOX_BORDER_WIDTH, 1.)
        .adding(TEXTBOX_INSETS, Insets::new(4.0, 2.0, 4.0, 2.0))
        .adding(SCROLLBAR_COLOR, Color::rgb8(0xff, 0xff, 0xff))
        .adding(SCROLLBAR_BORDER_COLOR, Color::rgb8(0x77, 0x77, 0x77))
        .adding(SCROLLBAR_MAX_OPACITY, 0.7)
        .adding(SCROLLBAR_FADE_DELAY, 1500u64)
        .adding(SCROLLBAR_WIDTH, 8.)
        .adding(SCROLLBAR_PAD, 2.)
        .adding(SCROLLBAR_RADIUS, 5.)
        .adding(SCROLLBAR_EDGE_WIDTH, 1.)
        .adding(WIDGET_PADDING_VERTICAL, 10.0)
        .adding(WIDGET_PADDING_HORIZONTAL, 8.0)
        .adding(WIDGET_CONTROL_COMPONENT_PADDING, 4.0)
        .adding(
            UI_FONT,
            FontDescriptor::new(FontFamily::SYSTEM_UI).with_size(15.0),
        )
        .adding(
            UI_FONT_BOLD,
            FontDescriptor::new(FontFamily::SYSTEM_UI)
                .with_weight(FontWeight::BOLD)
                .with_size(15.0),
        )
        .adding(
            UI_FONT_ITALIC,
            FontDescriptor::new(FontFamily::SYSTEM_UI)
                .with_style(FontStyle::Italic)
                .with_size(15.0),
        )
}

#[deprecated(since = "0.7.0", note = "use Env::default() instead")]
pub fn init() -> Env {
    Env::default()
}
