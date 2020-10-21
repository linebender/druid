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

//! An example of a blocking function running in another thread. We give
//! the other thread some data and then we also pass some data back
//! to the main thread using commands.

use std::{thread, time};

use druid::widget::prelude::*;
use druid::widget::{Button, Either, Flex, Label, Spinner};
use druid::{
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, ExtEventSink, Handled, Lens,
    LocalizedString, Selector, Target, WidgetExt, WindowDesc,
};

const FINISH_SLOW_FUNCTION: Selector<u32> = Selector::new("finish_slow_function");

#[derive(Clone, Default, Data, Lens)]
struct AppState {
    processing: bool,
    value: u32,
}

fn ui_builder() -> impl Widget<AppState> {
    let button = Button::new("Start slow increment")
        .on_click(|ctx, data: &mut AppState, _env| {
            data.processing = true;
            // In order to make sure that the other thread can communicate with the main thread we
            // have to pass an external handle to the second thread.
            // Using this handle we can send commands back to the main thread.
            wrapped_slow_function(ctx.get_external_handle(), data.value);
        })
        .padding(5.0);

    let button_placeholder = Flex::column()
        .with_child(Label::new(LocalizedString::new("Processing...")).padding(5.0))
        .with_child(Spinner::new());

    // Hello-counter is defined in the built-in localisation file. This maps to "Current value is {count}"
    // localised in english, french, or german. Every time the value is updated it shows the new value.
    let text = LocalizedString::new("hello-counter")
        .with_arg("count", |data: &AppState, _env| (data.value).into());
    let label = Label::new(text).padding(5.0).center();

    let either = Either::new(|data, _env| data.processing, button_placeholder, button);

    Flex::column().with_child(label).with_child(either)
}

fn wrapped_slow_function(sink: ExtEventSink, number: u32) {
    thread::spawn(move || {
        let number = slow_function(number);
        // Once the slow function is done we can use the event sink (the external handle).
        // This sends the `FINISH_SLOW_FUNCTION` command to the main thread and attach
        // the number as payload.
        sink.submit_command(FINISH_SLOW_FUNCTION, number, Target::Auto)
            .expect("command failed to submit");
    });
}

// Pretend this is downloading a file, or doing heavy calculations...
fn slow_function(number: u32) -> u32 {
    let a_while = time::Duration::from_millis(2000);
    thread::sleep(a_while);
    number + 1
}

struct Delegate;

impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppState,
        _env: &Env,
    ) -> Handled {
        if let Some(number) = cmd.get(FINISH_SLOW_FUNCTION) {
            // If the command we received is `FINISH_SLOW_FUNCTION` handle the payload.
            data.processing = false;
            data.value = *number;
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

fn main() {
    let main_window = WindowDesc::new(ui_builder).title(LocalizedString::new("Blocking functions"));
    AppLauncher::with_window(main_window)
        .delegate(Delegate {})
        .launch(AppState::default())
        .expect("launch failed");
}
