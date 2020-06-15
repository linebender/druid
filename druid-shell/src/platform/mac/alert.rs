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

//! macOS alert dialog implementation.

#![allow(non_upper_case_globals)]

use std::ffi::c_void;

use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSInteger};
use objc::rc::WeakPtr;
use objc::runtime::Object;
use objc::{msg_send, sel, sel_impl};

use crate::alert::{AlertIcon, AlertRequest, AlertResponse};

use super::appkit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSAlertStyle,
    NSAlertThirdButtonReturn, NSButton, NSControl, NSModalResponseAbort, NSModalResponseCancel,
    NSModalResponseStop,
};
use super::util::make_nsstring;
use super::window::ViewState;

// Derive our own constants to allow for clearer code, especially without a cancel button.
const BUTTON_ID_PRIMARY: NSInteger = NSAlertFirstButtonReturn;
const BUTTON_ID_CANCEL: NSInteger = NSAlertSecondButtonReturn;
const BUTTON_ID_ALT_START: NSInteger = NSAlertThirdButtonReturn;

pub(crate) unsafe fn show(view: &mut Object, request: AlertRequest) {
    let alert = NSAlert::alloc(nil).init().autorelease();

    alert.setMessageText(make_nsstring(&request.options.message));
    alert.setInformativeText(make_nsstring(&request.options.description));

    match request.options.icon {
        Some(AlertIcon::Information) => alert.setAlertStyle(NSAlertStyle::Informational),
        Some(AlertIcon::Warning) => alert.setAlertStyle(NSAlertStyle::Warning),
        Some(AlertIcon::Error) => alert.setAlertStyle(NSAlertStyle::Critical),
        None => (), // The OS will default to NSAlertStyle::Warning
    }

    // The primary button is always first. (macOS alert buttons go from right to left)
    let btn_primary = alert.addButtonWithTitle(make_nsstring(&request.options.primary.label));
    // Set the primary button as the default which can be chosen via Enter.
    btn_primary.setKeyEquivalent(make_nsstring("\r"));
    btn_primary.setTag(BUTTON_ID_PRIMARY);

    // If there's a cancel button, it is always second.
    if let Some(cancel) = &request.options.cancel {
        let btn_cancel = alert.addButtonWithTitle(make_nsstring(&cancel.label));
        // Mark it as the cancel button by adding the Escape hotkey. This also activates Cmd+dot.
        btn_cancel.setKeyEquivalent(make_nsstring("\u{1b}"));
        btn_cancel.setTag(BUTTON_ID_CANCEL);
    }

    // Any alternative buttons are always last.
    for (idx, alternative) in request.options.alternatives.iter().enumerate() {
        let btn_alt = alert.addButtonWithTitle(make_nsstring(&alternative.label));
        // Any hotkeys need to be explicit with custom buttons to avoid duplicates.
        btn_alt.setKeyEquivalent(make_nsstring(""));
        btn_alt.setTag(BUTTON_ID_ALT_START + idx as NSInteger);
    }

    if request.options.app_modal {
        let tag = alert.runModal();
        handle_result(view, &request, tag);
    } else {
        let window: id = msg_send![view, window];
        let view_ptr = WeakPtr::new(view as *mut Object);

        let alert_handler = block::ConcreteBlock::new(move |tag: NSInteger| {
            let view = *view_ptr.load();
            if !view.is_null() {
                handle_result(&mut *view, &request, tag);
            }
        });
        let alert_handler = alert_handler.copy(); // Force the block to live on the heap

        alert.beginSheetModalForWindow_completionHandler(window, alert_handler);
    }
}

unsafe fn handle_result(view: &mut Object, request: &AlertRequest, tag: NSInteger) {
    let button = match tag {
        BUTTON_ID_PRIMARY => Some(request.options.primary.clone()),
        BUTTON_ID_CANCEL | NSModalResponseAbort | NSModalResponseStop | NSModalResponseCancel => {
            None
        }
        id => {
            if id >= BUTTON_ID_ALT_START
                && id < BUTTON_ID_ALT_START + request.options.alternatives.len() as NSInteger
            {
                Some(request.options.alternatives[(id - BUTTON_ID_ALT_START) as usize].clone())
            } else {
                log::error!("Unexpected alert dialog result {}", id);
                None
            }
        }
    };
    let view_state = {
        let view_state: *mut c_void = *view.get_ivar("viewState");
        &mut *(view_state as *mut ViewState)
    };
    (*view_state)
        .handler
        .alert_response(AlertResponse::new(request.token, button));
}
