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

use crate::{Env, FontDescriptor, FontFamily, FontStyle, FontWeight, Insets, Key, key};

pub const WINDOW_BACKGROUND_COLOR: Key<Color> =
       key!("org.linebender.druid.theme.window_background_color", Color::rgb8(0x29, 0x29, 0x29));

pub const LABEL_COLOR: Key<Color> = key!("org.linebender.druid.theme.label_color", Color::rgb8(0xf0, 0xf0, 0xea));
pub const PLACEHOLDER_COLOR: Key<Color> = key!("org.linebender.druid.theme.placeholder_color", Color::rgb8(0x80, 0x80, 0x80));

pub const PRIMARY_LIGHT: Key<Color> = key!("org.linebender.druid.theme.primary_light", Color::rgb8(0x5c, 0xc4, 0xff));
pub const PRIMARY_DARK: Key<Color> = key!("org.linebender.druid.theme.primary_dark", Color::rgb8(0x00, 0x8d, 0xdd));
pub const PROGRESS_BAR_RADIUS: Key<f64> =
    key!("org.linebender.druid.theme.progress_bar_radius", 4.);
pub const BACKGROUND_LIGHT: Key<Color> = key!("org.linebender.druid.theme.background_light", Color::rgb8(0x3a, 0x3a, 0x3a));
pub const BACKGROUND_DARK: Key<Color> = key!("org.linebender.druid.theme.background_dark", Color::rgb8(0x31, 0x31, 0x31));
pub const FOREGROUND_LIGHT: Key<Color> = key!("org.linebender.druid.theme.foreground_light", Color::rgb8(0xf9, 0xf9, 0xf9));
pub const FOREGROUND_DARK: Key<Color> = key!("org.linebender.druid.theme.foreground_dark", Color::rgb8(0xbf, 0xbf, 0xbf));
pub const BUTTON_DARK: Key<Color> = key!("org.linebender.druid.theme.button_dark", Color::BLACK);
pub const BUTTON_LIGHT: Key<Color> = key!("org.linebender.druid.theme.button_light", Color::rgb8(0x21, 0x21, 0x21));
pub const BUTTON_BORDER_RADIUS: Key<f64> = key!("org.linebender.druid.theme.button_radius", 4.);
pub const BUTTON_BORDER_WIDTH: Key<f64> =
    key!("org.linebender.druid.theme.button_border_width", 2.);
pub const BORDER_DARK: Key<Color> = key!("org.linebender.druid.theme.border_dark", Color::rgb8(0x3a, 0x3a, 0x3a));
pub const BORDER_LIGHT: Key<Color> = key!("org.linebender.druid.theme.border_light", Color::rgb8(0xa1, 0xa1, 0xa1));
#[deprecated(since = "0.8.0", note = "use SELECTED_TEXT_BACKGROUND_COLOR instead")]
pub const SELECTION_COLOR: Key<Color> = SELECTED_TEXT_BACKGROUND_COLOR;
pub const SELECTED_TEXT_BACKGROUND_COLOR: Key<Color> =
    key!("org.linebender.druid.theme.selection_color", Color::rgb8(0x43, 0x70, 0xA8));
pub const SELECTED_TEXT_INACTIVE_BACKGROUND_COLOR: Key<Color> =
    key!("org.linebender.druid.theme.selection_color_inactive", Color::grey8(0x74));
pub const SELECTION_TEXT_COLOR: Key<Color> =
    key!("org.linebender.druid.theme.selection_text_color", Color::rgb8(0x00, 0x00, 0x00));
pub const CURSOR_COLOR: Key<Color> = key!("org.linebender.druid.theme.cursor_color", Color::WHITE);

pub const TEXT_SIZE_NORMAL: Key<f64> = key!("org.linebender.druid.theme.text_size_normal", 15.0);
pub const TEXT_SIZE_LARGE: Key<f64> = key!("org.linebender.druid.theme.text_size_large", 24.0);
pub const BASIC_WIDGET_HEIGHT: Key<f64> =
    key!("org.linebender.druid.theme.basic_widget_height", 18.0);

/// The default font for labels, buttons, text boxes, and other UI elements.
pub const UI_FONT: Key<FontDescriptor> = key!("org.linebender.druid.theme.ui-font", FontDescriptor::new(FontFamily::SYSTEM_UI).with_size(15.0));

/// A bold version of the default UI font.
pub const UI_FONT_BOLD: Key<FontDescriptor> = key!("org.linebender.druid.theme.ui-font-bold", FontDescriptor::new(FontFamily::SYSTEM_UI)
                .with_weight(FontWeight::BOLD)
                .with_size(15.0));

/// An Italic version of the default UI font.
pub const UI_FONT_ITALIC: Key<FontDescriptor> =
    key!("org.linebender.druid.theme.ui-font-italic", FontDescriptor::new(FontFamily::SYSTEM_UI)
                .with_style(FontStyle::Italic)
                .with_size(15.0));

/// The default minimum width for a 'wide' widget; a textbox, slider, progress bar, etc.
pub const WIDE_WIDGET_WIDTH: Key<f64> = key!("org.linebender.druid.theme.long-widget-width", 100.);
pub const BORDERED_WIDGET_HEIGHT: Key<f64> =
    key!("org.linebender.druid.theme.bordered_widget_height", 24.0);

pub const TEXTBOX_BORDER_RADIUS: Key<f64> =
    key!("org.linebender.druid.theme.textbox_border_radius", 2.);
pub const TEXTBOX_BORDER_WIDTH: Key<f64> =
    key!("org.linebender.druid.theme.textbox_border_width", 1.);
pub const TEXTBOX_INSETS: Key<Insets> = key!("org.linebender.druid.theme.textbox_insets", Insets::new(4.0, 4.0, 4.0, 4.0));

/// The default horizontal spacing between widgets.
pub const WIDGET_PADDING_HORIZONTAL: Key<f64> =
    key!("org.linebender.druid.theme.widget-padding-h", 8.0);
/// The default vertical spacing between widgets.
pub const WIDGET_PADDING_VERTICAL: Key<f64> =
    key!("org.linebender.druid.theme.widget-padding-v", 10.0);
/// The default internal (horizontal) padding for visually distinct components
/// of a widget; for instance between a checkbox and its label.
pub const WIDGET_CONTROL_COMPONENT_PADDING: Key<f64> =
    key!("org.linebender.druid.theme.widget-padding-control-label", 4.0);

pub const SCROLLBAR_COLOR: Key<Color> = key!("org.linebender.druid.theme.scrollbar_color", Color::rgb8(0xff, 0xff, 0xff));
pub const SCROLLBAR_BORDER_COLOR: Key<Color> =
    key!("org.linebender.druid.theme.scrollbar_border_color", Color::rgb8(0x77, 0x77, 0x77));
pub const SCROLLBAR_MAX_OPACITY: Key<f64> =
    key!("org.linebender.druid.theme.scrollbar_max_opacity", 0.7);
pub const SCROLLBAR_FADE_DELAY: Key<u64> =
    key!("org.linebender.druid.theme.scrollbar_fade_time", 1500u64);
pub const SCROLLBAR_WIDTH: Key<f64> = key!("org.linebender.druid.theme.scrollbar_width", 8.);
pub const SCROLLBAR_PAD: Key<f64> = key!("org.linebender.druid.theme.scrollbar_pad", 2.);
pub const SCROLLBAR_RADIUS: Key<f64> = key!("org.linebender.druid.theme.scrollbar_radius", 5.);
pub const SCROLLBAR_EDGE_WIDTH: Key<f64> =
    key!("org.linebender.druid.theme.scrollbar_edge_width", 1.);
/// Minimum length for any scrollbar to be when measured on that
/// scrollbar's primary axis.
pub const SCROLLBAR_MIN_SIZE: Key<f64> = key!("org.linebender.theme.scrollbar_min_size", 45.);

#[deprecated(since = "0.7.0", note = "use Env::default() instead")]
pub fn init() -> Env {
    Env::default()
}
