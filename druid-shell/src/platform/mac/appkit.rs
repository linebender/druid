// Copyright 2020 The xi-editor Authors.
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

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use bitflags::bitflags;
use block::RcBlock;
use cocoa::base::id;
use cocoa::foundation::{NSInteger, NSRect};
use objc::{class, msg_send, sel, sel_impl};

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

pub trait NSButton: Sized {
    unsafe fn setKeyEquivalent(self, key: id);
}

impl NSButton for id {
    unsafe fn setKeyEquivalent(self, key: id) {
        msg_send![self, setKeyEquivalent: key]
    }
}

pub trait NSControl: Sized {
    unsafe fn setTag(self, tag: NSInteger);
}

impl NSControl for id {
    unsafe fn setTag(self, tag: NSInteger) {
        msg_send![self, setTag: tag]
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NSAlertStyle {
    Warning,
    Informational,
    Critical,
}

pub const NSModalResponseContinue: NSInteger = -1002;
pub const NSModalResponseAbort: NSInteger = -1001;
pub const NSModalResponseStop: NSInteger = -1000;
pub const NSModalResponseCancel: NSInteger = 0;
pub const NSModalResponseOK: NSInteger = 1;

pub const NSAlertFirstButtonReturn: NSInteger = 1000;
pub const NSAlertSecondButtonReturn: NSInteger = 1001;
pub const NSAlertThirdButtonReturn: NSInteger = 1002;

pub trait NSAlert: Sized {
    unsafe fn alloc(_: Self) -> id {
        msg_send![class!(NSAlert), alloc]
    }

    unsafe fn init(self) -> id;
    unsafe fn setAlertStyle(self, style: NSAlertStyle);
    unsafe fn runModal(self) -> NSInteger;
    unsafe fn beginSheetModalForWindow_completionHandler(
        self,
        window: id,
        completionHandler: RcBlock<(NSInteger,), ()>,
    );
    unsafe fn setInformativeText(self, informativeText: id);
    unsafe fn setMessageText(self, messageText: id);
    unsafe fn addButtonWithTitle(self, title: id) -> id;
    unsafe fn window(self) -> id;
}

impl NSAlert for id {
    unsafe fn init(self) -> id {
        msg_send![self, init]
    }

    unsafe fn setAlertStyle(self, alertStyle: NSAlertStyle) {
        msg_send![self, setAlertStyle: alertStyle]
    }

    unsafe fn runModal(self) -> NSInteger {
        msg_send![self, runModal]
    }

    unsafe fn beginSheetModalForWindow_completionHandler(
        self,
        window: id,
        completionHandler: RcBlock<(NSInteger,), ()>,
    ) {
        msg_send![self, beginSheetModalForWindow: window completionHandler: completionHandler]
    }

    unsafe fn setInformativeText(self, informativeText: id) {
        msg_send![self, setInformativeText: informativeText]
    }

    unsafe fn setMessageText(self, messageText: id) {
        msg_send![self, setMessageText: messageText]
    }

    unsafe fn addButtonWithTitle(self, title: id) -> id {
        msg_send![self, addButtonWithTitle: title]
    }

    unsafe fn window(self) -> id {
        msg_send![self, window]
    }
}
