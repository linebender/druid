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

mod formatters;
mod widgets;

use druid::widget::{prelude::*, Flex, Label, TextBox};
use druid::{AppLauncher, Data, Lens, WidgetExt, WidgetId, WindowDesc};

use formatters::{
    CanadianPostalCodeFormatter, CatSelectingFormatter, NaiveCurrencyFormatter, PostalCode,
};
use widgets::{RootController, TextBoxErrorDelegate};

/// Various values that we are going to use with formatters.
#[derive(Debug, Clone, Data, Lens)]
pub struct AppData {
    dollars: f64,
    euros: f64,
    pounds: f64,
    postal_code: PostalCode,
    dont_type_cat: String,
    #[data(ignore)]
    active_textbox: Option<WidgetId>,
    active_message: Option<&'static str>,
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder()).title("Formatting and Validation");

    let data = AppData {
        dollars: 12.2,
        euros: -20.0,
        pounds: 1337.,
        postal_code: PostalCode::new("H0H0H0").unwrap(),
        dont_type_cat: String::new(),
        active_textbox: None,
        active_message: None,
    };

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(data)
        .expect("launch failed");
}

fn dollar_validation_scope() -> impl Widget<AppData> {
    let textbox = TextBox::new()
        .with_formatter(NaiveCurrencyFormatter::DOLLARS)
        .validate_while_editing(false)
        .delegate(
            TextBoxErrorDelegate::new(widgets::DOLLAR_ERROR_WIDGET).sends_partial_errors(true),
        );
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
        .with_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
                .with_child(Label::new("Dollars:"))
                .with_default_spacer()
                .with_child(textbox),
        )
        .with_child(widgets::error_display_widget(widgets::DOLLAR_ERROR_WIDGET))
        .lens(AppData::dollars)
}

fn euro_validation_scope() -> impl Widget<AppData> {
    let textbox = TextBox::new()
        .with_formatter(NaiveCurrencyFormatter::EUROS)
        .delegate(TextBoxErrorDelegate::new(widgets::EURO_ERROR_WIDGET).sends_partial_errors(true));
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
        .with_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
                .with_child(Label::new("Euros, often:"))
                .with_default_spacer()
                .with_child(textbox),
        )
        .with_child(widgets::error_display_widget(widgets::EURO_ERROR_WIDGET))
        .lens(AppData::euros)
}

fn pound_validation_scope() -> impl Widget<AppData> {
    let textbox = TextBox::new()
        .with_formatter(NaiveCurrencyFormatter::GBP)
        .update_data_while_editing(true)
        .delegate(
            TextBoxErrorDelegate::new(widgets::POUND_ERROR_WIDGET).sends_partial_errors(true),
        );
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
        .with_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
                .with_child(Label::new("Sterling Quidpence:"))
                .with_default_spacer()
                .with_child(textbox),
        )
        .with_child(widgets::error_display_widget(widgets::POUND_ERROR_WIDGET))
        .lens(AppData::pounds)
}

fn postal_validation_scope() -> impl Widget<AppData> {
    let textbox = TextBox::new()
        .with_formatter(CanadianPostalCodeFormatter)
        .delegate(
            TextBoxErrorDelegate::new(widgets::POSTAL_ERROR_WIDGET).sends_partial_errors(true),
        );
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
        .with_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
                .with_child(Label::new("Postal code:"))
                .with_default_spacer()
                .with_child(textbox),
        )
        .with_child(widgets::error_display_widget(widgets::POSTAL_ERROR_WIDGET))
        .lens(AppData::postal_code)
}

fn cat_validation_scope() -> impl Widget<AppData> {
    let textbox = TextBox::new()
        .with_placeholder("^(`.`)^")
        .with_formatter(CatSelectingFormatter)
        .delegate(TextBoxErrorDelegate::new(widgets::CAT_ERROR_WIDGET));
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
        .with_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
                .with_child(Label::new("Cat selector:"))
                .with_default_spacer()
                .with_child(textbox),
        )
        .lens(AppData::dont_type_cat)
}

fn ui_builder() -> impl Widget<AppData> {
    Flex::column()
        .with_child(
            widgets::explainer()
                .padding(10.0)
                .border(druid::theme::BORDER_DARK, 4.0)
                .rounded(10.0)
                .padding(10.0),
        )
        .with_default_spacer()
        .with_child(
            Flex::column()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
                .with_child(dollar_validation_scope())
                .with_default_spacer()
                .with_child(euro_validation_scope())
                .with_default_spacer()
                .with_child(pound_validation_scope())
                .with_default_spacer()
                .with_child(postal_validation_scope())
                .with_default_spacer()
                .with_child(cat_validation_scope())
                .center(),
        )
        .with_default_spacer()
        .with_child(
            widgets::active_value()
                .padding(10.0)
                .border(druid::theme::BORDER_DARK, 4.0)
                .rounded(10.0)
                .padding(10.0),
        )
        .controller(RootController)
}
