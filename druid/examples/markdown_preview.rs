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

use pulldown_cmark::{Event as ParseEvent, Parser, Tag};

use druid::text::{AttributesAdder, RichText, RichTextBuilder};
use druid::widget::prelude::*;
use druid::widget::{Controller, LineBreaking, RawLabel, Scroll, Split, TextBox};
use druid::{
    AppDelegate, AppLauncher, Color, Command, Data, DelegateCtx, FontFamily, FontStyle, FontWeight,
    Handled, Lens, LocalizedString, Menu, Selector, Target, Widget, WidgetExt, WindowDesc,
    WindowId,
};

const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Minimal Markdown");

const TEXT: &str = "*Hello* ***world***! This is a `TextBox` where you can \
		    use limited markdown notation, which is reflected in the \
		    **styling** of the `Label` on the left.

		    If you're curious about Druid, a good place to ask questions \
		    and discuss development work is our [Zulip chat instance], \
		    in the #druid-help and #druid channels, respectively.\n\n\n\
		    
		    [Zulip chat instance]: https://xi.zulipchat.com";

const SPACER_SIZE: f64 = 8.0;
const BLOCKQUOTE_COLOR: Color = Color::grey8(0x88);
const LINK_COLOR: Color = Color::rgb8(0, 0, 0xEE);
const OPEN_LINK: Selector<String> = Selector::new("druid-example.open-link");

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

struct Delegate;

impl<T: Data> AppDelegate<T> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        _data: &mut T,
        _env: &Env,
    ) -> Handled {
        if let Some(url) = cmd.get(OPEN_LINK) {
            #[cfg(not(target_arch = "wasm32"))]
            open::that_in_background(url);
            #[cfg(target_arch = "wasm32")]
            tracing::warn!("opening link({}) not supported on web yet.", url);
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .menu(make_menu)
        .window_size((700.0, 600.0));

    // create the initial app state
    let initial_state = AppState {
        raw: TEXT.to_owned(),
        rendered: rebuild_rendered_text(TEXT),
    };

    // start the application
    AppLauncher::with_window(main_window)
        .log_to_console()
        .delegate(Delegate)
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
    let mut builder = RichTextBuilder::new();
    let mut tag_stack = Vec::new();

    let parser = Parser::new(text);
    for event in parser {
        match event {
            ParseEvent::Start(tag) => {
                tag_stack.push((current_pos, tag));
            }
            ParseEvent::Text(txt) => {
                builder.push(&txt);
                current_pos += txt.len();
            }
            ParseEvent::End(end_tag) => {
                let (start_off, tag) = tag_stack
                    .pop()
                    .expect("parser does not return unbalanced tags");
                assert_eq!(end_tag, tag, "mismatched tags?");
                add_attribute_for_tag(
                    &tag,
                    builder.add_attributes_for_range(start_off..current_pos),
                );
                if add_newline_after_tag(&tag) {
                    builder.push("\n\n");
                    current_pos += 2;
                }
            }
            ParseEvent::Code(txt) => {
                builder.push(&txt).font_family(FontFamily::MONOSPACE);
                current_pos += txt.len();
            }
            ParseEvent::Html(txt) => {
                builder
                    .push(&txt)
                    .font_family(FontFamily::MONOSPACE)
                    .text_color(BLOCKQUOTE_COLOR);
                current_pos += txt.len();
            }
            ParseEvent::HardBreak => {
                builder.push("\n\n");
                current_pos += 2;
            }
            _ => (),
        }
    }
    builder.build()
}

fn add_newline_after_tag(tag: &Tag) -> bool {
    !matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link(..)
    )
}

fn add_attribute_for_tag(tag: &Tag, mut attrs: AttributesAdder) {
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
            attrs.size(font_size).weight(FontWeight::BOLD);
        }
        Tag::BlockQuote => {
            attrs.style(FontStyle::Italic).text_color(BLOCKQUOTE_COLOR);
        }
        Tag::CodeBlock(_) => {
            attrs.font_family(FontFamily::MONOSPACE);
        }
        Tag::Emphasis => {
            attrs.style(FontStyle::Italic);
        }
        Tag::Strong => {
            attrs.weight(FontWeight::BOLD);
        }
        Tag::Link(_link_ty, target, _title) => {
            attrs
                .underline(true)
                .text_color(LINK_COLOR)
                .link(OPEN_LINK.with(target.to_string()));
        }
        // ignore other tags for now
        _ => (),
    }
}

#[allow(unused_assignments, unused_mut)]
fn make_menu<T: Data>(_window_id: Option<WindowId>, _app_state: &AppState, _env: &Env) -> Menu<T> {
    let mut base = Menu::empty();
    #[cfg(target_os = "macos")]
    {
        base = base.entry(druid::platform_menus::mac::application::default())
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        base = base.entry(druid::platform_menus::win::file::default());
    }
    base.entry(
        Menu::new(LocalizedString::new("common-menu-edit-menu"))
            .entry(druid::platform_menus::common::undo())
            .entry(druid::platform_menus::common::redo())
            .separator()
            .entry(druid::platform_menus::common::cut().enabled(false))
            .entry(druid::platform_menus::common::copy())
            .entry(druid::platform_menus::common::paste()),
    )
}
