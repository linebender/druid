// Copyright 2021 The Druid Authors.
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

// https://github.com/dtolnay/trybuild
// trybuild is a crate that essentially runs cargo on the provided files, and checks the output.
// Tests may suddenly fail after a new compiler release, and there's not much we can do about that.
// If the test suite fails because of trybuild:
// - Update your compiler to the latest stable version.
// - If it still fails, update the stderr snapshots. To do so, run the test suite with
// env variable TRYBUILD=overwrite, and submit the file changes in a PR.
use trybuild::TestCases;

#[test]
fn ui() {
    let t = TestCases::new();
    t.pass("tests/ui/simple-lens.rs");
    t.pass("tests/ui/lens-attributes.rs");
    t.compile_fail("tests/ui/with-empty-struct.rs");
    t.compile_fail("tests/ui/with-tuple-struct.rs");
    t.compile_fail("tests/ui/with-enum.rs");
    t.compile_fail("tests/ui/with-union.rs");

    t.compile_fail("tests/ui/with-snake_case.rs");
}
