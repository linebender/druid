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

mod accels;
pub mod application;
pub mod clipboard;
pub mod dcomp;
pub mod dialog;
pub mod error;
pub mod keycodes;
pub mod menu;
pub mod paint;
mod timers;
pub mod util;
pub mod window;

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
// A Device Context, ID2D1DeviceContext, is available as of windows 7 platform update. This
// is the the minimum compatibility target for druid. We are not making an effort to do
// RenderTarget only.
//
// Basically, go from HwndRenderTarget or DxgiSurfaceRenderTarget (2d or 3d) to a Device Context.
// Go back up for particular needs.

use piet_common::d2d::{D2DFactory, DeviceContext};
use std::fmt::{Debug, Display, Formatter};
use winapi::shared::windef::HWND;
use winapi::shared::winerror::{HRESULT, SUCCEEDED};
use winapi::um::d2d1::{
    ID2D1HwndRenderTarget, ID2D1RenderTarget, D2D1_HWND_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_SIZE_U,
};
use winapi::um::dcommon::D2D1_PIXEL_FORMAT;
use wio::com::ComPtr;

#[derive(Clone)]
pub struct HwndRenderTarget {
    ptr: ComPtr<ID2D1HwndRenderTarget>,
}

impl HwndRenderTarget {
    pub fn create(
        factory: &D2DFactory,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> Result<Self, Error> {
        // hardcode
        // - RenderTargetType::Default
        // - AlphaMode::Unknown
        let rt_props = DEFAULT_PROPS;
        let mut hwnd_props = DEFAULT_HWND_PROPS;

        hwnd_props.hwnd = hwnd;
        hwnd_props.pixelSize.width = width;
        hwnd_props.pixelSize.height = height;

        // now build
        unsafe {
            let mut ptr = std::ptr::null_mut();
            let hr = (*factory.get_raw()).CreateHwndRenderTarget(&rt_props, &hwnd_props, &mut ptr);

            if SUCCEEDED(hr) {
                Ok(HwndRenderTarget::from_raw(ptr))
            } else {
                Err(hr.into())
            }
        }
    }

    /// construct from COM ptr
    ///
    /// # Safety
    /// TODO
    pub unsafe fn from_ptr(ptr: ComPtr<ID2D1HwndRenderTarget>) -> Self {
        Self { ptr }
    }

    /// construct from raw ptr
    ///
    /// # Safety
    /// TODO
    pub unsafe fn from_raw(raw: *mut ID2D1HwndRenderTarget) -> Self {
        Self::from_ptr(ComPtr::from_raw(raw))
    }

    pub fn get_comptr(&self) -> &ComPtr<ID2D1HwndRenderTarget> {
        &self.ptr
    }
}

// props for creating hwnd render target
const DEFAULT_PROPS: D2D1_RENDER_TARGET_PROPERTIES = D2D1_RENDER_TARGET_PROPERTIES {
    _type: 0u32, //RenderTargetType::Default
    pixelFormat: D2D1_PIXEL_FORMAT {
        format: 87u32, //Format::B8G8R8A8Unorm, see https://docs.rs/dxgi/0.3.0-alpha4/src/dxgi/enums/format.rs.html#631
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
    ptr: ComPtr<ID2D1RenderTarget>,
}

impl DxgiSurfaceRenderTarget {
    /// construct from raw ptr
    ///
    /// # Safety
    /// TODO
    pub unsafe fn from_raw(raw: *mut ID2D1RenderTarget) -> Self {
        DxgiSurfaceRenderTarget {
            ptr: ComPtr::from_raw(raw),
        }
    }

    /// cast to DeviceContext
    ///
    /// # Safety
    /// TODO
    pub unsafe fn as_device_context(&self) -> Option<DeviceContext> {
        self.ptr
            .cast()
            .ok()
            .map(|com_ptr| DeviceContext::new(com_ptr))
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
