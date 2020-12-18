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

//! [CVDisplayLink](https://developer.apple.com/documentation/corevideo/cvdisplaylink?language=objc)

use std::ffi::c_void;

use cocoa::quartzcore::CVTimeStamp;

#[repr(C)]
pub struct __CVDisplayLink;

pub type CVDisplayLinkRef = *mut __CVDisplayLink;

pub type CVDisplayLinkOutputCallback = unsafe extern "C" fn(
    display_link_out: CVDisplayLinkRef,
    in_now_timestamp: *const CVTimeStamp,
    in_output_timestamp: *const CVTimeStamp,
    flags_in: i64,
    flagsOut: *mut i64,
    display_link_context: *mut c_void,
) -> i32;

#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "CoreVideo", kind = "framework")]
#[allow(improper_ctypes)]
extern "C" {
    pub fn CVDisplayLinkCreateWithActiveCGDisplays(display_link_out: *mut CVDisplayLinkRef) -> i32;
    pub fn CVDisplayLinkSetOutputCallback(
        display_link: CVDisplayLinkRef,
        callback: CVDisplayLinkOutputCallback,
        user_info: *mut c_void,
    ) -> i32;
    pub fn CVDisplayLinkStart(display_link: CVDisplayLinkRef) -> i32;
    pub fn CVDisplayLinkStop(display_link: CVDisplayLinkRef) -> i32;
    pub fn CVDisplayLinkRelease(display_link: CVDisplayLinkRef);
    pub fn CVDisplayLinkRetain(display_link: CVDisplayLinkRef) -> CVDisplayLinkRef;
}

pub struct CVDisplayLink(pub CVDisplayLinkRef);

impl Drop for CVDisplayLink {
    fn drop(&mut self) {
        unsafe { CVDisplayLinkRelease(self.0) }
    }
}

impl Clone for CVDisplayLink {
    fn clone(&self) -> Self {
        unsafe { Self(CVDisplayLinkRetain(self.0)) }
    }
}

impl CVDisplayLink {
    pub unsafe fn with_active_cg_displays() -> Self {
        let mut out = std::ptr::null_mut();
        assert_eq!(CVDisplayLinkCreateWithActiveCGDisplays(&mut out), 0);
        Self(out)
    }

    pub unsafe fn set_output_callback(
        &self,
        callback: CVDisplayLinkOutputCallback,
        user_info: *mut c_void,
    ) {
        assert_eq!(
            CVDisplayLinkSetOutputCallback(self.0, callback, user_info),
            0
        );
    }

    pub unsafe fn start(&self) {
        CVDisplayLinkStart(self.0);
    }

    pub unsafe fn stop(&self) {
        CVDisplayLinkStop(self.0);
    }
}
