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

use sha2::{Digest, Sha256};
use xilem::{async_list, scroll_view, App, AppLauncher, View};

fn compute_hash(i: usize) -> String {
    let mut s = format!("{}", i);
    for _ in 0..i {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        s = hex::encode(result);
    }
    s
}

fn app_logic(_: &mut ()) -> impl View<()> {
    scroll_view(async_list(10_000, 16.0, |i| async move {
        format!("{}: {}", i, compute_hash(i))
    }))
}

#[tokio::main]
async fn main() {
    let app = App::new((), app_logic);
    AppLauncher::new(app).run();
}
