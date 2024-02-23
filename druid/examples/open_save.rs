// Copyright 2020 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Usage of file open and saving.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use druid::widget::{Align, Button, Flex, TextBox};
use druid::{
    commands, AppDelegate, AppLauncher, Command, DelegateCtx, Env, FileDialogOptions, FileSpec,
    Handled, LocalizedString, Target, Widget, WindowDesc,
};

struct Delegate;

pub fn main() {
    let main_window = WindowDesc::new(ui_builder())
        .title(LocalizedString::new("open-save-demo").with_placeholder("Opening/Saving Demo"));
    let data = "Type here.".to_owned();
    AppLauncher::with_window(main_window)
        .delegate(Delegate)
        .log_to_console()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<String> {
    let rs = FileSpec::new("Rust source", &["rs"]);
    let txt = FileSpec::new("Text file", &["txt"]);
    let other = FileSpec::new("Bogus file", &["foo", "bar", "baz"]);
    // The options can also be generated at runtime,
    // so to show that off we create a String for the default save name.
    let default_save_name = String::from("MyFile.txt");
    let save_dialog_options = FileDialogOptions::new()
        .allowed_types(vec![rs, txt, other])
        .default_type(txt)
        .default_name(default_save_name)
        .name_label("Target")
        .title("Choose a target for this lovely file")
        .button_text("Export");
    let open_dialog_options = save_dialog_options
        .clone()
        .default_name("MySavedFile.txt")
        .name_label("Source")
        .title("Where did you put that file?")
        .button_text("Import");

    let input = TextBox::new();
    let save = Button::new("Save").on_click(move |ctx, _, _| {
        ctx.submit_command(druid::commands::SHOW_SAVE_PANEL.with(save_dialog_options.clone()))
    });
    let open = Button::new("Open").on_click(move |ctx, _, _| {
        ctx.submit_command(druid::commands::SHOW_OPEN_PANEL.with(open_dialog_options.clone()))
    });

    let mut col = Flex::column();
    col.add_child(input);
    col.add_spacer(8.0);
    col.add_child(save);
    col.add_child(open);
    Align::centered(col)
}

impl AppDelegate<String> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut String,
        _env: &Env,
    ) -> Handled {
        if let Some(file_info) = cmd.get(commands::SAVE_FILE_AS) {
            if let Err(e) = std::fs::write(file_info.path(), &data[..]) {
                println!("Error writing file: {e}");
            }
            return Handled::Yes;
        }
        if let Some(file_info) = cmd.get(commands::OPEN_FILE) {
            match std::fs::read_to_string(file_info.path()) {
                Ok(s) => {
                    let first_line = s.lines().next().unwrap_or("");
                    *data = first_line.to_owned();
                }
                Err(e) => {
                    println!("Error opening file: {e}");
                }
            }
            return Handled::Yes;
        }
        Handled::No
    }
}
