// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Widgets, widget components, and functions for creating widgets

use druid::text::ValidationError;
use druid::widget::{
    prelude::*, Controller, Either, Label, SizedBox, TextBoxEvent, ValidationDelegate,
};
use druid::{Color, Data, Point, Selector, WidgetExt, WidgetId, WidgetPod};

use super::AppData;

pub const DOLLAR_ERROR_WIDGET: WidgetId = WidgetId::reserved(2);
pub const EURO_ERROR_WIDGET: WidgetId = WidgetId::reserved(3);
pub const POUND_ERROR_WIDGET: WidgetId = WidgetId::reserved(4);
pub const POSTAL_ERROR_WIDGET: WidgetId = WidgetId::reserved(5);
pub const CAT_ERROR_WIDGET: WidgetId = WidgetId::reserved(6);

/// Sent by the [`TextBoxErrorDelegate`] when an error should be displayed.
const SHOW_ERROR: Selector<ValidationError> = Selector::new("druid-example.show-error");
/// Sent by the [`TextBoxErrorDelegate`] when an error should be cleared.
const CLEAR_ERROR: Selector = Selector::new("druid-example.clear-error");
/// Sent by the [`TextBoxErrorDelegate`] when editing began.
///
/// This is used to set the contents of the help text.
const EDIT_BEGAN: Selector<WidgetId> = Selector::new("druid-example.edit-began");
/// Sent by the [`TextBoxErrorDelegate`] when editing finishes.
///
/// This is used to set the contents of the help text.
const EDIT_FINISHED: Selector<WidgetId> = Selector::new("druid-example.edit-finished");

static DEFAULT_EXPLAINER: &str = "This example shows various ways you can use a \
                                 TextBox with a Formatter to control the display \
                                 and validation of text.";

static DOLLAR_EXPLAINER: &str = "This text field accepts any input, and performs \
                                 validation when the user attempts to complete \
                                 editing.";

static EURO_EXPLAINER: &str = "This text field performs validation during editing, \
                               rejecting invalid edits.";

static POUND_EXPLAINER: &str = "This text field updates the application data \
                                during editing, whenever the current input is \
                                valid.";

static POSTAL_EXPLAINER: &str = "This text field edits the contents to ensure \
                                 all letters are capitalized, and the two triplets \
                                 are space-separated.";

static CAT_EXPLAINER: &str = "This text field does no formatting, but selects any \
                              instance of the string 'cat'.";

const ERROR_TEXT_COLOR: Color = Color::rgb8(0xB6, 0x00, 0x04);

/// Create a widget that will display errors.
///
/// The `id` param is the `WidgetId` that this widget should use; that id
/// will be sent messages when it should display or clear an error.
pub fn error_display_widget<T: Data>(id: WidgetId) -> impl Widget<T> {
    ErrorController::new(
        Either::new(
            |d: &Option<ValidationError>, _| d.is_some(),
            Label::dynamic(|d: &Option<ValidationError>, _| {
                d.as_ref().map(|d| d.to_string()).unwrap_or_default()
            })
            .with_text_color(ERROR_TEXT_COLOR)
            .with_text_size(12.0),
            SizedBox::empty(),
        )
        .with_id(id),
    )
}

/// Create the 'explainer' widget.
pub fn explainer() -> impl Widget<AppData> {
    Label::dynamic(|d: &Option<&'static str>, _| d.unwrap_or(DEFAULT_EXPLAINER).to_string())
        .with_line_break_mode(druid::widget::LineBreaking::WordWrap)
        .with_text_color(druid::theme::FOREGROUND_DARK)
        .with_font(druid::theme::UI_FONT_ITALIC)
        .lens(AppData::active_message)
}

/// Create the 'active value' widget
pub fn active_value() -> impl Widget<AppData> {
    Label::dynamic(|d: &AppData, _| match d.active_textbox {
        None => "No textfield is active".to_string(),
        Some(id) => match id {
            DOLLAR_ERROR_WIDGET => format!("Active value: {:2}", d.dollars),
            EURO_ERROR_WIDGET => format!("Active value: {:2}", d.euros),
            POUND_ERROR_WIDGET => format!("Active value: {:2}", d.pounds),
            POSTAL_ERROR_WIDGET => format!("Active value: {}", d.postal_code),
            CAT_ERROR_WIDGET => format!("Active value: {}", d.dont_type_cat),
            _ => unreachable!(),
        },
    })
    .with_text_color(druid::theme::FOREGROUND_DARK)
    .with_font(druid::theme::UI_FONT_ITALIC)
}

/// A widget that manages a child which can display an error.
///
/// This is not a blessed pattern, but works around certain limitations of Druid,
/// using Commands.
///
/// The basic idea is that this widget owns an `Option<Error>`, and it either
/// clears or sets this error based on `Command`s sent to it from some other
/// widget.
///
/// Its child's data is this `Option<Error>`; the incoming data is ignored
/// completely.
pub struct ErrorController<W> {
    child: WidgetPod<Option<ValidationError>, W>,
    error: Option<ValidationError>,
}

/// A controller that sits at the root of our widget tree and updates the
/// helper text in response to messages about the currently focused widget.
pub struct RootController;

pub struct TextBoxErrorDelegate {
    target: WidgetId,
    sends_partial_errors: bool,
}

impl<W: Widget<Option<ValidationError>>> ErrorController<W> {
    pub fn new(child: W) -> ErrorController<W> {
        ErrorController {
            child: WidgetPod::new(child),
            error: None,
        }
    }
}

impl<T, W: Widget<Option<ValidationError>>> Widget<T> for ErrorController<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, env: &Env) {
        match event {
            Event::Command(cmd) if cmd.is(SHOW_ERROR) => {
                self.error = Some(cmd.get_unchecked(SHOW_ERROR).to_owned());
                ctx.request_update();
            }
            Event::Command(cmd) if cmd.is(CLEAR_ERROR) => {
                self.error = None;
                ctx.request_update();
            }
            _ => self.child.event(ctx, event, &mut self.error, env),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _: &T, env: &Env) {
        self.child.lifecycle(ctx, event, &self.error, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _: &T, _: &T, env: &Env) {
        self.child.update(ctx, &self.error, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _: &T, env: &Env) -> Size {
        let size = self.child.layout(ctx, bc, &self.error, env);
        self.child.set_origin(ctx, Point::ZERO);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _: &T, env: &Env) {
        self.child.paint(ctx, &self.error, env);
    }

    fn id(&self) -> Option<WidgetId> {
        Some(self.child.id())
    }
}

impl<W: Widget<AppData>> Controller<AppData, W> for RootController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppData,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(EDIT_BEGAN) => {
                let widget_id = *cmd.get_unchecked(EDIT_BEGAN);
                data.active_message = match widget_id {
                    DOLLAR_ERROR_WIDGET => Some(DOLLAR_EXPLAINER),
                    EURO_ERROR_WIDGET => Some(EURO_EXPLAINER),
                    POUND_ERROR_WIDGET => Some(POUND_EXPLAINER),
                    POSTAL_ERROR_WIDGET => Some(POSTAL_EXPLAINER),
                    CAT_ERROR_WIDGET => Some(CAT_EXPLAINER),
                    _ => unreachable!(),
                };
                data.active_textbox = Some(widget_id);
            }
            Event::Command(cmd) if cmd.is(EDIT_FINISHED) => {
                let finished_id = *cmd.get_unchecked(EDIT_FINISHED);
                if data.active_textbox == Some(finished_id) {
                    data.active_textbox = None;
                    data.active_message = None;
                }
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

impl TextBoxErrorDelegate {
    pub fn new(target: WidgetId) -> TextBoxErrorDelegate {
        TextBoxErrorDelegate {
            target,
            sends_partial_errors: false,
        }
    }

    pub fn sends_partial_errors(mut self, flag: bool) -> Self {
        self.sends_partial_errors = flag;
        self
    }
}

impl ValidationDelegate for TextBoxErrorDelegate {
    fn event(&mut self, ctx: &mut EventCtx, event: TextBoxEvent, _current_text: &str) {
        match event {
            TextBoxEvent::Began => {
                ctx.submit_command(CLEAR_ERROR.to(self.target));
                ctx.submit_command(EDIT_BEGAN.with(self.target));
            }
            TextBoxEvent::Changed if self.sends_partial_errors => {
                ctx.submit_command(CLEAR_ERROR.to(self.target));
            }
            TextBoxEvent::PartiallyInvalid(err) if self.sends_partial_errors => {
                ctx.submit_command(SHOW_ERROR.with(err).to(self.target));
            }
            TextBoxEvent::Invalid(err) => {
                ctx.submit_command(SHOW_ERROR.with(err).to(self.target));
            }
            TextBoxEvent::Cancel | TextBoxEvent::Complete => {
                ctx.submit_command(CLEAR_ERROR.to(self.target));
                ctx.submit_command(EDIT_FINISHED.with(self.target));
            }
            _ => (),
        }
    }
}
