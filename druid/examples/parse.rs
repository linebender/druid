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

//! Demonstrates how to use value formatters to constrain the contents
//! of a text box.

use druid::text::format::{Formatter, Validation, ValidationError};
use druid::text::{Selection, TextLayout};
use druid::widget::{prelude::*, Flex, Label, TextBox};
use druid::{AppLauncher, Data, Lens, Point, Selector, WidgetExt, WidgetPod, WindowDesc};

/// Various values that we are going to use with formatters.
#[derive(Debug, Clone, Data, Lens)]
struct AppData {
    dollars: f64,
    euros: f64,
    pounds: f64,
    postal_code: PostalCode,
    dont_type_cat: String,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder).title("Formatting and Validation");

    let data = AppData {
        dollars: 12.2,
        euros: -20.0,
        pounds: 1337.,
        postal_code: PostalCode::new("H0H0H0").unwrap(),
        dont_type_cat: String::new(),
    };

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
        .with_child(
            error_displaying_value_textbox("Dollars:", NaiveCurrencyFormatter::DOLLARS, None, None)
                .lens(AppData::dollars),
        )
        .with_default_spacer()
        .with_child(
            error_displaying_value_textbox(
                "Euros, often:",
                NaiveCurrencyFormatter::EUROS,
                None,
                None,
            )
            .lens(AppData::euros),
        )
        .with_default_spacer()
        .with_child(
            error_displaying_value_textbox(
                "Sterling Quidpence:",
                NaiveCurrencyFormatter::GBP,
                None,
                None,
            )
            .lens(AppData::pounds),
        )
        .with_default_spacer()
        .with_child(
            error_displaying_value_textbox(
                "Postal Code:",
                CanadianPostalCodeFormatter,
                Some("H1M 0M0"),
                None,
            )
            .lens(AppData::postal_code),
        )
        .with_default_spacer()
        .with_child(
            error_displaying_value_textbox(
                "Cat selector:",
                CatSelectingFormatter,
                Some("Don't type 'cat'"),
                Some(140.0),
            )
            .lens(AppData::dont_type_cat),
        )
        .center()
        //.debug_paint_layout()
        .debug_widget_id()
}

fn error_displaying_value_textbox<T: Data>(
    label: &str,
    formatter: impl Formatter<T> + 'static,
    placeholder: Option<&str>,
    textbox_width: Option<f64>,
) -> impl Widget<T> {
    const DEFAULT_WIDTH: f64 = 100.0;
    let label = Label::new(label);
    let textbox = TextBox::new()
        .with_placeholder(placeholder.unwrap_or(""))
        .with_formatter(formatter)
        .fix_width(textbox_width.unwrap_or(DEFAULT_WIDTH));

    ErrorDisplayTextbox::new(
        Flex::row()
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
            .with_child(label)
            .with_default_spacer()
            .with_child(textbox),
    )
}

struct ErrorDisplayTextbox<T, W> {
    textbox: WidgetPod<T, W>,
    error: Option<TextLayout<String>>,
}

impl<T: Data, W: Widget<T>> ErrorDisplayTextbox<T, W> {
    fn new(textbox: W) -> Self {
        ErrorDisplayTextbox {
            textbox: WidgetPod::new(textbox),
            error: None,
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for ErrorDisplayTextbox<T, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::Notification(n) if n.is(TextBox::VALIDATION_FAILED) => {
                let text = n.get(TextBox::VALIDATION_FAILED).unwrap().to_string();
                let mut layout = TextLayout::from_text(text);
                layout.set_text_size(12.0);
                layout.set_text_alignment(druid::TextAlignment::End);
                self.error = Some(layout);
                ctx.set_handled();
            }
            Event::Notification(n) if n.is(TextBox::EDITING_FINISHED) => {
                self.error = None;
                ctx.request_layout();
                ctx.set_handled();
            }
            _ => self.textbox.event(ctx, event, data, env),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.textbox.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.textbox.update(ctx, data, env);
        if !old_data.same(data) {
            self.error = None;
        } else if let Some(error) = &mut self.error {
            if error.needs_rebuild_after_update(ctx) {
                ctx.request_layout();
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let textbox_size = self.textbox.layout(ctx, bc, data, env);
        self.textbox.set_origin(ctx, data, env, Point::ZERO);
        match &mut self.error {
            Some(error) => {
                error.set_wrap_width(textbox_size.width);
                error.rebuild_if_needed(ctx.text(), env);
                let error_size = error.size();
                let total_width = textbox_size.width.max(error_size.width);
                let v_padding = env.get(druid::theme::WIDGET_CONTROL_COMPONENT_PADDING);
                let total_height = textbox_size.height + error_size.height + v_padding;
                // set our baseline; we want to use the baseline of the TextBox.
                let textbox_baseline = self.textbox.baseline_offset();
                let our_baseline = textbox_baseline + v_padding + error_size.height;
                ctx.set_baseline_offset(our_baseline);
                Size::new(total_width, total_height)
            }
            None => {
                // if we aren't drawing the label, our baseline is identical to textbox's
                ctx.set_baseline_offset(self.textbox.baseline_offset());
                textbox_size
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let v_padding = env.get(druid::theme::WIDGET_CONTROL_COMPONENT_PADDING);
        self.textbox.paint(ctx, data, env);
        let textbox_size = self.textbox.layout_rect().size();
        if let Some(error) = &mut self.error {
            // we align the error to the right edge of the textbox, if allowed
            let h_padding = (textbox_size.width - error.size().width).max(0.0);

            let error_pos = Point::new(h_padding, self.textbox.layout_rect().height() + v_padding);
            error.draw(ctx, error_pos);
        }
    }
}

/// A formatter that can display currency values.
struct NaiveCurrencyFormatter {
    currency_symbol: char,
    thousands_separator: char,
    decimal_separator: char,
}

/// Errors returned by [`NaiveCurrencyFormatter`].
#[derive(Debug, Clone)]
enum CurrencyValidationError {
    Parse(std::num::ParseFloatError),
    InvalidChar(char),
    TooManyCharsAfterDecimal,
}

/// A `Formatter` for postal codes, which are the format A0A 0A0 where 'A' is
/// any uppercase ascii character, and '0' is any numeral.
///
/// This formatter will accept lowercase characters as input, but will replace
/// them with uppercase characters.
struct CanadianPostalCodeFormatter;

/// A Canadian postal code, in the format 'A0A0A0'.
#[derive(Debug, Clone, Copy, Data)]
struct PostalCode {
    chars: [u8; 6],
}

/// Error returned by [`CanadianPostalCodeFormatter`].
#[derive(Debug, Clone)]
enum PostalCodeValidationError {
    WrongNumberOfCharacters,
    IncorrectFormat,
}

/// A formatter that sets the selection to the first occurance of the word 'cat'
/// in an input string, if it is found.
struct CatSelectingFormatter;

impl NaiveCurrencyFormatter {
    const DOLLARS: NaiveCurrencyFormatter = NaiveCurrencyFormatter {
        currency_symbol: '$',
        thousands_separator: ',',
        decimal_separator: '.',
    };

    const EUROS: NaiveCurrencyFormatter = NaiveCurrencyFormatter {
        currency_symbol: '€',
        thousands_separator: '.',
        decimal_separator: ',',
    };

    const GBP: NaiveCurrencyFormatter = NaiveCurrencyFormatter {
        currency_symbol: '£',
        thousands_separator: '.',
        decimal_separator: ',',
    };
}

impl Formatter<f64> for NaiveCurrencyFormatter {
    fn format(&self, value: &f64) -> String {
        if !value.is_normal() {
            return format!("{}0{}00", self.currency_symbol, self.decimal_separator);
        }

        let mut components = Vec::new();
        let mut major_part = value.abs().trunc() as usize;
        let minor_part = (value.abs().fract() * 100.0).round() as usize;

        let bonus_rounding_dollar = minor_part / 100;

        components.push(format!("{}{:02}", self.decimal_separator, minor_part % 100));
        if major_part == 0 {
            components.push('0'.to_string());
        }

        while major_part > 0 {
            let remain = major_part % 1000;
            major_part /= 1000;
            if major_part > 0 {
                components.push(format!("{}{:03}", self.thousands_separator, remain));
            } else {
                components.push((remain + bonus_rounding_dollar).to_string());
            }
        }
        if value.is_sign_negative() {
            components.push(format!("-{}", self.currency_symbol));
        } else {
            components.push(self.currency_symbol.to_string());
        }

        components.iter().rev().flat_map(|s| s.chars()).collect()
    }

    fn format_for_editing(&self, value: &f64) -> String {
        self.format(value)
            .chars()
            .filter(|c| *c != self.currency_symbol)
            .collect()
    }

    fn value(&self, input: &str) -> Result<f64, ValidationError> {
        // we need to convert from our naive localized representation back into
        // rust's float representation
        let decimal_pos = input
            .bytes()
            .rposition(|b| b as char == self.decimal_separator);
        let (major, minor) = input.split_at(decimal_pos.unwrap_or_else(|| input.len()));
        let canonical: String = major
            .chars()
            .filter(|c| *c != self.thousands_separator)
            .chain(Some('.'))
            .chain(minor.chars().skip(1))
            .collect();
        canonical
            .parse()
            .map_err(|err| ValidationError::new(CurrencyValidationError::Parse(err)))
    }

    fn validate_partial_input(&self, input: &str, _sel: &Selection) -> Validation {
        if input.is_empty() {
            return Validation::success();
        }

        let mut char_iter = input.chars();
        if let Some(c) = char_iter.next() {
            if !(c.is_ascii_digit() || c == '-') {
                return Validation::failure();
            }
        }
        let mut char_iter =
            char_iter.skip_while(|c| c.is_ascii_digit() || *c == self.thousands_separator);
        match char_iter.next() {
            None => return Validation::success(),
            Some(c) if c == self.decimal_separator => (),
            Some(c) => return Validation::failure(CurrencyValidationError::InvalidChar(c)),
        };

        // we're after the decimal, allow up to 2 digits
        let (d1, d2, d3) = (char_iter.next(), char_iter.next(), char_iter.next());
        match (d1, d2, d3) {
            (_, _, Some(_)) => {
                Validation::failure(CurrencyValidationError::TooManyCharsAfterDecimal)
            }
            (Some(c), None, _) if c.is_ascii_digit() => Validation::success(),
            (None, None, _) => Validation::success(),
            (Some(c1), Some(c2), None) if c1.is_ascii_digit() && c2.is_ascii_digit() => {
                Validation::success()
            }
            (Some(c1), other, _) => {
                let bad_char = if c1.is_ascii_digit() {
                    c1
                } else {
                    other.unwrap()
                };
                Validation::failure(CurrencyValidationError::InvalidChar(bad_char))
            }
            _ => unreachable!(),
        }
    }
}

//TODO: never write parsing code like this again
impl Formatter<PostalCode> for CanadianPostalCodeFormatter {
    fn format(&self, value: &PostalCode) -> String {
        value.to_string()
    }

    fn validate_partial_input(&self, input: &str, sel: &Selection) -> Validation {
        let mut chars = input.chars();
        let mut valid = true;
        let mut has_space = false;
        if matches!(chars.next(), Some(c) if !c.is_ascii_alphabetic()) {
            valid = false;
        }
        if matches!(chars.next(), Some(c) if !c.is_ascii_digit()) {
            valid = false;
        }
        if matches!(chars.next(), Some(c) if !c.is_ascii_alphabetic()) {
            valid = false;
        }
        match chars.next() {
            Some(' ') => {
                has_space = true;
                if matches!(chars.next(), Some(c) if !c.is_ascii_digit()) {
                    valid = false;
                }
            }
            Some(other) if !other.is_ascii_digit() => valid = false,
            _ => (),
        }

        if matches!(chars.next(), Some(c) if !c.is_ascii_alphabetic()) {
            valid = false;
        }

        if matches!(chars.next(), Some(c) if !c.is_ascii_digit()) {
            valid = false;
        }

        if chars.next().is_some() {
            valid = false;
        }

        if valid {
            // if valid we convert to canonical format; h1h2h2 becomes H!H 2H2
            let (replacement_text, sel) = if input.len() < 4 || has_space {
                (input.to_uppercase(), None)
            } else {
                //let split_at = 3.min(input.len().saturating_sub(1));
                let (first, second) = input.split_at(3);
                let insert_space = if second.bytes().next() == Some(b' ') {
                    None
                } else {
                    Some(' ')
                };
                let sel = if insert_space.is_some() && sel.is_caret() {
                    Some(Selection::caret(sel.min() + 1))
                } else {
                    None
                };
                (
                    first
                        .chars()
                        .map(|c| c.to_ascii_uppercase())
                        .chain(insert_space)
                        .chain(second.chars().map(|c| c.to_ascii_uppercase()))
                        .collect(),
                    sel,
                )
            };

            if let Some(replacement_sel) = sel {
                Validation::success()
                    .change_text(replacement_text)
                    .change_selection(replacement_sel)
            } else {
                Validation::success().change_text(replacement_text)
            }
        } else {
            Validation::failure(ValidationError::new(
                PostalCodeValidationError::IncorrectFormat,
            ))
        }
    }

    #[allow(clippy::clippy::many_single_char_names, clippy::clippy::match_ref_pats)]
    fn value(&self, input: &str) -> Result<PostalCode, ValidationError> {
        match input.as_bytes() {
            &[a, b, c, d, e, f] => PostalCode::from_bytes([a, b, c, d, e, f]),
            &[a, b, c, b' ', d, e, f] => PostalCode::from_bytes([a, b, c, d, e, f]),
            _ => Err(PostalCodeValidationError::WrongNumberOfCharacters),
        }
        .map_err(ValidationError::new)
    }
}

impl PostalCode {
    fn new(s: &str) -> Result<Self, PostalCodeValidationError> {
        if s.as_bytes().len() != 6 {
            Err(PostalCodeValidationError::WrongNumberOfCharacters)
        } else {
            let b = s.as_bytes();
            Self::from_bytes([b[0], b[1], b[2], b[3], b[4], b[5]])
        }
    }

    fn from_bytes(bytes: [u8; 6]) -> Result<Self, PostalCodeValidationError> {
        if [bytes[0], bytes[2], bytes[4]]
            .iter()
            .all(|b| (*b as char).is_ascii_uppercase())
            && [bytes[1], bytes[3], bytes[5]]
                .iter()
                .all(|b| (*b as char).is_ascii_digit())
        {
            Ok(PostalCode { chars: bytes })
        } else {
            Err(PostalCodeValidationError::IncorrectFormat)
        }
    }
}

#[allow(clippy::clippy::many_single_char_names)]
impl std::fmt::Display for PostalCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let [a, b, c, d, e, g] = self.chars;
        write!(
            f,
            "{}{}{} {}{}{}",
            a as char, b as char, c as char, d as char, e as char, g as char
        )
    }
}

impl Formatter<String> for CatSelectingFormatter {
    fn format(&self, value: &String) -> String {
        value.to_owned()
    }

    fn value(&self, input: &str) -> Result<String, ValidationError> {
        Ok(input.to_owned())
    }

    fn validate_partial_input(&self, input: &str, _sel: &Selection) -> Validation {
        if let Some(idx) = input.find("cat") {
            Validation::success().change_selection(Selection::new(idx, idx + 3))
        } else {
            Validation::success()
        }
    }
}

impl std::fmt::Display for CurrencyValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CurrencyValidationError::InvalidChar(c) => write!(f, "Invalid character '{}'", c),
            CurrencyValidationError::Parse(err) => write!(f, "Parse failed: {}", err),
            CurrencyValidationError::TooManyCharsAfterDecimal => {
                write!(f, "Too many characters after decimal")
            }
        }
    }
}

impl std::error::Error for CurrencyValidationError {}

impl std::fmt::Display for PostalCodeValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PostalCodeValidationError::WrongNumberOfCharacters => {
                write!(f, "Incorrect number of characters.")
            }
            PostalCodeValidationError::IncorrectFormat => {
                write!(f, "Postal code must be of format A2A2A2")
            }
        }
    }
}

impl std::error::Error for PostalCodeValidationError {}
