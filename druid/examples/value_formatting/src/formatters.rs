// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Implementations of the [`druid::text::Formatter`] trait.

use druid::text::{Formatter, Selection, Validation, ValidationError};
use druid::Data;

/// A formatter that can display currency values.
pub struct NaiveCurrencyFormatter {
    currency_symbol: char,
    thousands_separator: char,
    decimal_separator: char,
}

/// Errors returned by [`NaiveCurrencyFormatter`].
#[derive(Debug, Clone)]
pub enum CurrencyValidationError {
    Parse(std::num::ParseFloatError),
    InvalidChar(char),
    TooManyCharsAfterDecimal,
}

/// A `Formatter` for postal codes, which are the format A0A 0A0 where 'A' is
/// any uppercase ascii character, and '0' is any numeral.
///
/// This formatter will accept lowercase characters as input, but will replace
/// them with uppercase characters.
pub struct CanadianPostalCodeFormatter;

/// A Canadian postal code, in the format 'A0A0A0'.
#[derive(Debug, Clone, Copy, Data)]
pub struct PostalCode {
    chars: [u8; 6],
}

/// Error returned by [`CanadianPostalCodeFormatter`].
#[derive(Debug, Clone)]
pub enum PostalCodeValidationError {
    WrongNumberOfCharacters,
    IncorrectFormat,
}

/// A formatter that sets the selection to the first occurrence of the word 'cat'
/// in an input string, if it is found.
pub struct CatSelectingFormatter;

impl NaiveCurrencyFormatter {
    /// A formatter for USD.
    pub const DOLLARS: NaiveCurrencyFormatter = NaiveCurrencyFormatter {
        currency_symbol: '$',
        thousands_separator: ',',
        decimal_separator: '.',
    };

    /// A formatter for euros.
    pub const EUROS: NaiveCurrencyFormatter = NaiveCurrencyFormatter {
        currency_symbol: '€',
        thousands_separator: '.',
        decimal_separator: ',',
    };

    /// A formatter for british pounds.
    pub const GBP: NaiveCurrencyFormatter = NaiveCurrencyFormatter {
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
        let (major, minor) = input.split_at(decimal_pos.unwrap_or(input.len()));
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
                return Validation::failure(CurrencyValidationError::InvalidChar(c));
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
            (Some(c), None, _) => Validation::failure(CurrencyValidationError::InvalidChar(c)),
            (None, None, _) => Validation::success(),
            (Some(c1), Some(c2), _) if c1.is_ascii_digit() && c2.is_ascii_digit() => {
                Validation::success()
            }
            (Some(c1), Some(other), _) => {
                let bad_char = if c1.is_ascii_digit() { other } else { c1 };
                Validation::failure(CurrencyValidationError::InvalidChar(bad_char))
            }
            other => panic!("unexpected: {:?}", other),
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

    #[allow(clippy::many_single_char_names, clippy::match_ref_pats)]
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
    pub fn new(s: &str) -> Result<Self, PostalCodeValidationError> {
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

#[allow(clippy::many_single_char_names)]
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
