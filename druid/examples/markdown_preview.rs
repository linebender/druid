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

//! An example of live markdown preview

use std::ops::Range;

use pulldown_cmark::{Event as ParseEvent, Parser, Tag};

use druid::text::{Attribute, AttributeSpans, RichText};
use druid::widget::prelude::*;
use druid::widget::{Controller, LineBreaking, RawLabel, Scroll, Split, TextBox};
use druid::{
    AppLauncher, Color, Data, FontFamily, FontStyle, FontWeight, Lens, LocalizedString, MenuDesc,
    Widget, WidgetExt, WindowDesc,
};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Minimal Markdown");

const TEXT: &str = "*Hello* ***world***! This is a `TextBox` where you can \
                   use limited markdown notation, which is reflected in the \
                   **styling** of the `Label` on the left.";

const SPACER_SIZE: f64 = 8.0;
const BLOCKQUOTE_COLOR: Color = Color::grey8(0x88);
const LINK_COLOR: Color = Color::rgb8(0, 0, 0xEE);

#[derive(Clone, Data, Lens)]
struct AppState {
    raw: String,
    rendered: RichText,
}

/// A controller that rebuilds the preview when edits occur
struct RichTextRebuilder;

impl<W: Widget<AppState>> Controller<AppState, W> for RichTextRebuilder {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        let pre_data = data.raw.to_owned();
        child.event(ctx, event, data, env);
        if !data.raw.same(&pre_data) {
            data.rendered = rebuild_rendered_text(&data.raw);
        }
    }
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .menu(make_menu())
        .window_size((700.0, 600.0));

    // create the initial app state
    let initial_state = AppState {
        raw: TEXT.to_owned(),
        rendered: rebuild_rendered_text(TEXT),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<AppState> {
    let label = Scroll::new(
        RawLabel::new()
            .with_text_color(Color::BLACK)
            .with_line_break_mode(LineBreaking::WordWrap)
            .lens(AppState::rendered)
            .expand_width()
            .padding((SPACER_SIZE * 4.0, SPACER_SIZE)),
    )
    .vertical()
    .background(Color::grey8(222))
    .expand();

    let textbox = TextBox::multiline()
        .lens(AppState::raw)
        .controller(RichTextRebuilder)
        .expand()
        .padding(5.0);

    Split::columns(label, textbox)
}

/// Parse a markdown string and generate a `RichText` object with
/// the appropriate attributes.
fn rebuild_rendered_text(text: &str) -> RichText {
    let mut current_pos = 0;
    let mut buffer = String::new();
    let mut attrs = AttributeSpans::new();
    let mut tag_stack = Vec::new();

    let parser = Parser::new(text);
    for event in parser {
        match event {
            ParseEvent::Start(tag) => {
                tag_stack.push((current_pos, tag));
            }
            ParseEvent::Text(txt) => {
                buffer.push_str(&txt);
                current_pos += txt.len();
            }
            ParseEvent::End(end_tag) => {
                let (start_off, tag) = tag_stack
                    .pop()
                    .expect("parser does not return unbalanced tags");
                assert_eq!(end_tag, tag, "mismatched tags?");
                add_attribute_for_tag(&tag, start_off..current_pos, &mut attrs);
                if add_newline_after_tag(&tag) {
                    buffer.push_str("\n\n");
                    current_pos += 2;
                }
            }
            ParseEvent::Code(txt) => {
                buffer.push_str(&txt);
                let range = current_pos..current_pos + txt.len();
                attrs.add(range, Attribute::font_family(FontFamily::MONOSPACE));
                current_pos += txt.len();
            }
            ParseEvent::Html(txt) => {
                buffer.push_str(&txt);
                let range = current_pos..current_pos + txt.len();
                attrs.add(range.clone(), Attribute::font_family(FontFamily::MONOSPACE));
                attrs.add(range, Attribute::text_color(BLOCKQUOTE_COLOR));
                current_pos += txt.len();
            }
            ParseEvent::HardBreak => {
                buffer.push_str("\n\n");
                current_pos += 1;
            }
            _ => (),
        }
    }
    RichText::new_with_attributes(buffer.into(), attrs)
}

fn add_newline_after_tag(tag: &Tag) -> bool {
    !matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link(..)
    )
}

fn add_attribute_for_tag(tag: &Tag, range: Range<usize>, attrs: &mut AttributeSpans) {
    match tag {
        Tag::Heading(lvl) => {
            let font_size = match lvl {
                1 => 38.,
                2 => 32.0,
                3 => 26.0,
                4 => 20.0,
                5 => 16.0,
                _ => 12.0,
            };
            attrs.add(range.clone(), Attribute::size(font_size));
            attrs.add(range, Attribute::weight(FontWeight::BOLD));
        }
        Tag::BlockQuote => {
            attrs.add(range.clone(), Attribute::style(FontStyle::Italic));
            attrs.add(range, Attribute::text_color(BLOCKQUOTE_COLOR));
        }
        Tag::CodeBlock(_) => {
            attrs.add(range, Attribute::font_family(FontFamily::MONOSPACE));
        }
        Tag::Emphasis => attrs.add(range, Attribute::style(FontStyle::Italic)),
        Tag::Strong => attrs.add(range, Attribute::weight(FontWeight::BOLD)),
        Tag::Link(..) => {
            attrs.add(range.clone(), Attribute::underline(true));
            attrs.add(range, Attribute::text_color(LINK_COLOR));
        }
        // ignore other tags for now
        _ => (),
    }
}

#[allow(unused_assignments, unused_mut)]
fn make_menu<T: Data>() -> MenuDesc<T> {
    let mut base = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        base = base.append(druid::platform_menus::mac::application::default())
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        base = base.append(druid::platform_menus::win::file::default());
    }
    base.append(
        MenuDesc::new(LocalizedString::new("common-menu-edit-menu"))
            .append(druid::platform_menus::common::undo())
            .append(druid::platform_menus::common::redo())
            .append_separator()
            .append(druid::platform_menus::common::cut().disabled())
            .append(druid::platform_menus::common::copy())
            .append(druid::platform_menus::common::paste()),
    )
}
