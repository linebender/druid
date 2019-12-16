// Copyright 2017 The xi-editor Authors.
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

//! Bureaucracy to create render targets for painting.
//!
//! Note that these are currently implemented using hwnd render targets
//! because they are are (relatively) easy, but for high performance we want
//! dxgi render targets so we can use present options for minimal
//! invalidation and low-latency frame timing.

use std::mem;
use std::ptr::null_mut;

use log::{error, warn};

use winapi::ctypes::c_void;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;
use winapi::um::d2d1::*;
use winapi::um::dcommon::*;
use winapi::um::winuser::*;
use winapi::Interface;

// old
//use direct2d;
//use direct2d::enums::{AlphaMode, RenderTargetType};
//use direct2d::render_target::{DxgiSurfaceRenderTarget, GenericRenderTarget, HwndRenderTarget};

// new
use piet_common::d2d::D2DFactory;
use piet_common::dwrite::DwriteFactory;
use wio::com::ComPtr;

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
type HwndRenderTarget = ComPtr<ID2D1HwndRenderTarget>;
type DxgiSurfaceRenderTarget = ComPtr<ID2D1RenderTarget>;
type GenericRenderTarget = ComPtr<ID2D1RenderTarget>;
type DeviceContext = ComPtr<winapi::um::d2d1_1::ID2D1DeviceContext>;

// end new
use super::error::Error;
use super::util::as_result;

/// Context for painting by app into window.
pub struct PaintCtx<'a> {
    pub(crate) d2d_factory: &'a D2DFactory,
    pub(crate) render_target: &'a mut GenericRenderTarget,
}

pub(crate) unsafe fn create_render_target(
    d2d_factory: &D2DFactory,
    hwnd: HWND,
) -> Result<HwndRenderTarget, Error> {
    let mut rect: RECT = mem::zeroed();
    if GetClientRect(hwnd, &mut rect) == 0 {
        warn!("GetClientRect failed.");
        Err(Error::D2Error)
    } else {
        let width = (rect.right - rect.left) as u32;
        let height = (rect.bottom - rect.top) as u32;
        let res = HwndRenderTarget::create(d2d_factory)
            .with_hwnd(hwnd)
            .with_target_type(RenderTargetType::Default)
            .with_alpha_mode(AlphaMode::Unknown)
            .with_pixel_size(width, height)
            .build();
        if let Err(ref e) = res {
            error!("Creating hwnd render target failed: {:?}", e);
        }
        res.map_err(|_| Error::D2Error)
    }
}

/// Create a render target from a DXGI swapchain.
///
/// TODO: probably want to create a DeviceContext, it's more flexible.
pub(crate) unsafe fn create_render_target_dxgi(
    d2d_factory: &D2DFactory,
    swap_chain: *mut IDXGISwapChain1,
    dpi: f32,
) -> Result<DxgiSurfaceRenderTarget, Error> {
    let mut buffer: *mut IDXGISurface = null_mut();
    as_result((*swap_chain).GetBuffer(
        0,
        &IDXGISurface::uuidof(),
        &mut buffer as *mut _ as *mut *mut c_void,
    ))?;
    let props = D2D1_RENDER_TARGET_PROPERTIES {
        _type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_IGNORE,
        },
        dpiX: dpi,
        dpiY: dpi,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    };

    let mut render_target: *mut ID2D1RenderTarget = null_mut();
    let res =
        (*d2d_factory.get_raw()).CreateDxgiSurfaceRenderTarget(buffer, &props, &mut render_target);
    (*buffer).Release();
    if SUCCEEDED(res) {
        // TODO: maybe use builder
        Ok(DxgiSurfaceRenderTarget::from_raw(render_target))
    } else {
        Err(res.into())
    }
}

impl<'a> PaintCtx<'a> {
    /// Return the raw Direct2D factory for this painting context. Note: it's possible
    /// this will be wrapped to make it easier to port.
    pub fn d2d_factory(&self) -> &D2DFactory {
        self.d2d_factory
    }

    /// Return the raw Direct2D RenderTarget for this painting context. Note: it's possible
    /// this will be wrapped to make it easier to port.
    pub fn render_target(&mut self) -> &mut GenericRenderTarget {
        self.render_target
    }
}
