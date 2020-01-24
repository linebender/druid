// Copyright 2018 The xi-editor Authors.
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

//! Windows implementation of druid-shell.

pub mod application;
pub mod clipboard;
pub mod dcomp;
pub mod dialog;
pub mod error;
pub mod keycodes;
pub mod menu;
pub mod paint;
pub mod runloop;
mod timers;
pub mod util;
pub mod window;

// from winapi::um::d2d1;
// need to move from one type to another?
//
// https://docs.microsoft.com/en-us/windows/win32/direct2d/render-targets-overview
// ID2D1RenderTarget is the interface. The other resources inherit from it.
//
// A Render Target creates resources for drawing and performs drawing operations.
//
// - ID2D1HwndRenderTarget objects render content to a window.
// - ID2D1DCRenderTarget objects render to a GDI device context.
// - bitmap render target objects render to off-screen bitmap.
// - DXGI render target objects render to  a DXGI surface for use with Direct3D.
//
// https://docs.microsoft.com/en-us/windows/win32/direct2d/devices-and-device-contexts
// A Device Context, ID2D1DeviceContext, is used for windows 8 and higher. Render Target
// is used for windows 7 and lower.
//
// Basically, go from HwndRenderTarget or DxgiSurfaceRenderTarget (2d or 3d) to a Device Context.
// Go back up for particular needs. Move up and down using query_interface.
use piet_common::d2d::{D2DFactory, DeviceContext};
use std::fmt::{Debug, Display, Formatter};
use winapi::Interface;
use winapi::shared::windef::HWND;
use winapi::shared::winerror::{HRESULT, SUCCEEDED};
use winapi::um::d2d1::{D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_PROPERTIES,
                       D2D1_SIZE_U, ID2D1HwndRenderTarget, ID2D1RenderTarget};
use winapi::um::d2d1_1::ID2D1DeviceContext;
use winapi::um::dcommon::D2D1_PIXEL_FORMAT;
use wio::com::ComPtr;

// TODO make newtypes
// Can probably follow the pattern of direct2d crate
// e.g. https://github.com/Connicpu/direct2d-rs/blob/v0.1.2/src/render_target/hwnd.rs
//
// error and wrap, copy from d2d.rs
//
// for creating rendertarget, use
// https://github.com/hwchen/piet/blob/raw_d2d/piet-direct2d/src/d2d.rs#L165 as example, but fill
// in properties etc.

#[derive(Clone)]
pub struct HwndRenderTarget {
    ptr: ComPtr<ID2D1HwndRenderTarget>
}

impl HwndRenderTarget {
    pub fn create<'a>(factory: &'a D2DFactory, hwnd: HWND, width: u32, height: u32) -> Result<Self, Error> {
        // hardcode
        // - RenderTargetType::Default
        // - AlphaMode::Unknown
        let rt_props = DEFAULT_PROPS;
        let mut hwnd_props = DEFAULT_HWND_PROPS;

        hwnd_props.pixelSize.width = width;
        hwnd_props.pixelSize.height = height;

        // now build
        unsafe {
            let mut ptr = std::ptr::null_mut();
            let hr = (*factory.get_raw()).CreateHwndRenderTarget(
                &rt_props,
                &hwnd_props,
                &mut ptr,
            );

            if SUCCEEDED(hr) {
                Ok(HwndRenderTarget::from_raw(ptr))
            } else {
                Err(hr.into())
            }
        }
    }

    pub unsafe fn from_ptr(ptr: ComPtr<ID2D1HwndRenderTarget>) -> Self {
        Self { ptr }
    }

    pub unsafe fn from_raw(raw: *mut ID2D1HwndRenderTarget) -> Self {
        HwndRenderTarget {
            ptr: ComPtr::from_raw(raw),
        }
    }

    pub unsafe fn get_raw(&self) -> *mut ID2D1HwndRenderTarget {
        self.ptr.as_raw()
    }
}

// props for creating hwnd render target
const DEFAULT_PROPS: D2D1_RENDER_TARGET_PROPERTIES = D2D1_RENDER_TARGET_PROPERTIES {
    _type: 0u32, //RenderTargetType::Default
    pixelFormat: D2D1_PIXEL_FORMAT {
        format: 87u32,//Format::B8G8R8A8Unorm, see https://docs.rs/dxgi/0.3.0-alpha4/src/dxgi/enums/format.rs.html#631
        alphaMode: 0u32, //AlphaMode::Unknown
    },
    dpiX: 0.0,
    dpiY: 0.0,
    usage: 0,
    minLevel: 0,
};

const DEFAULT_HWND_PROPS: D2D1_HWND_RENDER_TARGET_PROPERTIES = D2D1_HWND_RENDER_TARGET_PROPERTIES {
    hwnd: std::ptr::null_mut(),
    pixelSize: D2D1_SIZE_U {
        width: 0,
        height: 0,
    },
    presentOptions: 0,
};

#[derive(Clone)]
pub struct DxgiSurfaceRenderTarget {
    ptr: ComPtr<ID2D1RenderTarget>
}

impl DxgiSurfaceRenderTarget {
    pub unsafe fn from_raw(raw: *mut ID2D1RenderTarget) -> Self {
        DxgiSurfaceRenderTarget {
            ptr: ComPtr::from_raw(raw),
        }
    }

    pub unsafe fn get_raw(&self) -> *mut ID2D1RenderTarget {
        self.ptr.as_raw()
    }

    // TODO use https://docs.rs/wio/0.2.2/i686-pc-windows-msvc/wio/com/struct.ComPtr.html#method.cast
    //  use something like
    //  ```
    //  self.ptr.cast().ok().map(DeviceContext)
    //  ```
    //  Note that DeviceContext needs a constructor, something like DeviceContext::from_com()
    //
    //
    //  NOTE for piet_common. for the pub structs, put docstrings that warn that it's windows only,
    //  used for platform-specific info. The four structs i'm exporting, the device context and the
    //  factories.
    //
    //  Make sure to rebase first for piet
    pub unsafe fn as_device_context(&self) -> Option<DeviceContext> {
        self.ptr.cast().ok().map(|com_ptr| DeviceContext::new(com_ptr))
    }
}

// error handling
pub enum Error {
    WinapiError(HRESULT),
}

impl From<HRESULT> for Error {
    fn from(hr: HRESULT) -> Error {
        Error::WinapiError(hr)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Error::WinapiError(hr) => write!(f, "hresult {:x}", hr),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Error::WinapiError(hr) => write!(f, "hresult {:x}", hr),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "winapi error"
    }
}

/// to wrap the result
unsafe fn wrap<T, U, F>(hr: HRESULT, ptr: *mut T, f: F) -> Result<U, Error>
where
    F: Fn(ComPtr<T>) -> U,
    T: Interface,
{
    if SUCCEEDED(hr) {
        Ok(f(ComPtr::from_raw(ptr)))
    } else {
        Err(hr.into())
    }
}

