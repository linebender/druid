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

//! macOS AppKit bindings.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use bitflags::bitflags;
use cocoa::base::id;
use cocoa::foundation::NSRect;
use objc::{class, msg_send, sel, sel_impl};

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    pub static NSRunLoopCommonModes: id;
}

bitflags! {
    pub struct NSTrackingAreaOptions: i32 {
        const MouseEnteredAndExited = 1;
        const MouseMoved = 1 << 1;
        const CursorUpdate = 1 << 2;
        // What's 1 << 3?
        const ActiveWhenFirstResponder = 1 << 4;
        const ActiveInKeyWindow = 1 << 5;
        const ActiveInActiveApp = 1 << 6;
        const ActiveAlways = 1 << 7;
        const AssumeInside = 1 << 8;
        const InVisibleRect = 1 << 9;
        const EnabledDuringMouseDrag = 1 << 10;
    }
}

pub trait NSTrackingArea: Sized {
    unsafe fn alloc(_: Self) -> id {
        msg_send![class!(NSTrackingArea), alloc]
    }

    unsafe fn initWithRect_options_owner_userInfo(
        self,
        rect: NSRect,
        options: NSTrackingAreaOptions,
        owner: id,
        userInfo: id,
    ) -> id;
}

impl NSTrackingArea for id {
    unsafe fn initWithRect_options_owner_userInfo(
        self,
        rect: NSRect,
        options: NSTrackingAreaOptions,
        owner: id,
        userInfo: id,
    ) -> id {
        msg_send![self, initWithRect:rect options:options owner:owner userInfo:userInfo]
    }
}

pub trait NSView: Sized {
    unsafe fn addTrackingArea(self, trackingArea: id) -> id;
}

impl NSView for id {
    unsafe fn addTrackingArea(self, trackingArea: id) -> id {
        msg_send![self, addTrackingArea: trackingArea]
    }
}
