// Copyright 2019 The xi-editor Authors.
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

use crate::piet::Color;

use crate::{Env, Key};

pub const COLOR_PRIMARY_LIGHT: Key<Color> = Key::new("color_primary_light");
pub const COLOR_PRIMARY: Key<Color> = Key::new("color_primary");

pub const COLOR_SECONDARY: Key<Color> = Key::new("color_secondary");

pub const COLOR_BASE_LIGHTEST: Key<Color> = Key::new("color_base_lightest");
pub const COLOR_BASE_LIGHTER: Key<Color> = Key::new("color_base_lighter");
pub const COLOR_BASE_LIGHT: Key<Color> = Key::new("color_base_light");
pub const COLOR_BASE: Key<Color> = Key::new("color_base");
pub const COLOR_BASE_DARK: Key<Color> = Key::new("color_base_dark");
pub const COLOR_BASE_DARKER: Key<Color> = Key::new("color_base_darker");
pub const COLOR_BASE_DARKEST: Key<Color> = Key::new("color_base_darkest");

pub const PROGRESS_BAR_RADIUS: Key<f64> = Key::new("progress_bar_radius");
pub const BUTTON_DARK: Key<Color> = Key::new("button_dark");
pub const BUTTON_LIGHT: Key<Color> = Key::new("button_light");
pub const BUTTON_BORDER_RADIUS: Key<f64> = Key::new("button_radius");
pub const BUTTON_BORDER_WIDTH: Key<f64> = Key::new("button_border_width");
pub const CURSOR_COLOR: Key<Color> = Key::new("cursor_color");

pub const FONT_NAME: Key<&str> = Key::new("font_name");
pub const TEXT_SIZE_NORMAL: Key<f64> = Key::new("text_size_normal");
pub const TEXT_SIZE_LARGE: Key<f64> = Key::new("text_size_large");
pub const BASIC_WIDGET_HEIGHT: Key<f64> = Key::new("basic_widget_height");

/// The default minimum width for a 'wide' widget; a textbox, slider, progress bar, etc.
pub const WIDE_WIDGET_WIDTH: Key<f64> = Key::new("druid.widgets.long-widget-width");
pub const BORDERED_WIDGET_HEIGHT: Key<f64> = Key::new("bordered_widget_height");

pub const TEXTBOX_BORDER_RADIUS: Key<f64> = Key::new("textbox_radius");

pub const SCROLL_BAR_COLOR: Key<Color> = Key::new("scroll_bar_color");
pub const SCROLL_BAR_BORDER_COLOR: Key<Color> = Key::new("scroll_bar_border_color");
pub const SCROLL_BAR_MAX_OPACITY: Key<f64> = Key::new("scroll_bar_max_opacity");
pub const SCROLL_BAR_FADE_DELAY: Key<u64> = Key::new("scroll_bar_fade_time");
pub const SCROLL_BAR_WIDTH: Key<f64> = Key::new("scroll_bar_width");
pub const SCROLL_BAR_PAD: Key<f64> = Key::new("scroll_bar_pad");
pub const SCROLL_BAR_RADIUS: Key<f64> = Key::new("scroll_bar_radius");
pub const SCROLL_BAR_EDGE_WIDTH: Key<f64> = Key::new("scroll_bar_edge_width");

/// An initial theme.
pub fn init() -> Env {
    let mut env = Env::default()
        .adding(COLOR_PRIMARY_LIGHT, Color::rgb8(0x5c, 0xc4, 0xff))
        .adding(COLOR_PRIMARY, Color::rgb8(0x00, 0x8d, 0xdd))
        .adding(COLOR_SECONDARY, Color::rgb8(0xf3, 0x00, 0x21))
        .adding(COLOR_BASE_LIGHTEST, Color::rgb8(0xe0, 0xe0, 0xe0))
        .adding(COLOR_BASE_LIGHTER, Color::rgb8(0xcf, 0xcf, 0xcf))
        .adding(COLOR_BASE_LIGHT, Color::rgb8(0xa9, 0xa9, 0xa9))
        .adding(COLOR_BASE, Color::rgb8(0x77, 0x77, 0x77))
        .adding(COLOR_BASE_DARK, Color::rgb8(0x56, 0x56, 0x56))
        .adding(COLOR_BASE_DARKER, Color::rgb8(0x3d, 0x3d, 0x3d))
        .adding(COLOR_BASE_DARKEST, Color::rgb8(0x29, 0x29, 0x29))
        .adding(PROGRESS_BAR_RADIUS, 4.)
        .adding(BUTTON_DARK, Color::BLACK)
        .adding(BUTTON_LIGHT, Color::rgb8(0x21, 0x21, 0x21))
        .adding(BUTTON_BORDER_RADIUS, 4.)
        .adding(BUTTON_BORDER_WIDTH, 2.)
        .adding(CURSOR_COLOR, Color::WHITE)
        .adding(TEXT_SIZE_NORMAL, 15.0)
        .adding(TEXT_SIZE_LARGE, 24.0)
        .adding(BASIC_WIDGET_HEIGHT, 18.0)
        .adding(WIDE_WIDGET_WIDTH, 100.)
        .adding(BORDERED_WIDGET_HEIGHT, 24.0)
        .adding(TEXTBOX_BORDER_RADIUS, 2.)
        .adding(SCROLL_BAR_COLOR, Color::rgb8(0xff, 0xff, 0xff))
        .adding(SCROLL_BAR_BORDER_COLOR, Color::rgb8(0x77, 0x77, 0x77))
        .adding(SCROLL_BAR_MAX_OPACITY, 0.7)
        .adding(SCROLL_BAR_FADE_DELAY, 1500u64)
        .adding(SCROLL_BAR_WIDTH, 8.)
        .adding(SCROLL_BAR_PAD, 2.)
        .adding(SCROLL_BAR_RADIUS, 5.)
        .adding(SCROLL_BAR_EDGE_WIDTH, 1.);

    #[cfg(target_os = "windows")]
    {
        env = env.adding(FONT_NAME, "Segoe UI");
    }
    #[cfg(target_os = "macos")]
    {
        // Ideally this would be a reference to San Francisco, but Cairo's
        // "toy text" API doesn't seem to be able to access it easily.
        env = env.adding(FONT_NAME, "Arial");
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        env = env.adding(FONT_NAME, "sans-serif");
    }
    env
}
