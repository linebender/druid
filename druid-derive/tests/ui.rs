// Copyright 2021 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

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
