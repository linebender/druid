#![allow(unused, non_upper_case_globals, non_camel_case_types)]
// unknown lints to make compile on older rust versions
#![cfg_attr(test, allow(unknown_lints, deref_nullptr))]
// generated code has some redudant static lifetimes, I don't think we can change that.
#![allow(clippy::redundant_static_lifetimes)]

use nix::libc::FILE;
include!(concat!(env!("OUT_DIR"), "/xkbcommon_sys.rs"));
