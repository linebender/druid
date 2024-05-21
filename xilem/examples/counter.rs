// Copyright 2022 The Druid Authors.
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

use xilem::{button, v_stack, Adapt, App, AppLauncher, LayoutObserver, Memoize, View};

#[derive(Default)]
struct AppData {
    count: u32,
}

fn count_button(count: u32) -> impl View<u32> {
    button(format!("count: {}", count), |data| *data += 1)
}

fn app_logic(data: &mut AppData) -> impl View<AppData> {
    v_stack((
        format!("count: {}", data.count),
        button("reset", |data: &mut AppData| data.count = 0),
        Memoize::new(data.count, |count| {
            button(format!("count: {}", count), |data: &mut AppData| {
                data.count += 1
            })
        }),
        Adapt::new(
            |data: &mut AppData, thunk| thunk.call(&mut data.count),
            count_button(data.count),
        ),
        LayoutObserver::new(|size| format!("size: {:?}", size)),
    ))
}

pub fn main() {
    let app = App::new(AppData::default(), app_logic);
    AppLauncher::new(app).run();
}
