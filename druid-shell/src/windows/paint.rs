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

use direct2d;
use direct2d::enums::{AlphaMode, RenderTargetType};
use direct2d::render_target::{DxgiSurfaceRenderTarget, GenericRenderTarget, HwndRenderTarget};

use crate::util::as_result;
use crate::Error;

/// Context for painting by app into window.
pub struct PaintCtx<'a> {
    pub(crate) d2d_factory: &'a direct2d::Factory,
    pub(crate) render_target: &'a mut GenericRenderTarget,
}

pub(crate) unsafe fn create_render_target(
    d2d_factory: &direct2d::Factory,
    hwnd: HWND,
) -> Result<HwndRenderTarget, Error> {
    let mut rect: RECT = mem::uninitialized();
    GetClientRect(hwnd, &mut rect);
    let width = (rect.right - rect.left) as u32;
    let height = (rect.bottom - rect.top) as u32;
    let res = HwndRenderTarget::create(d2d_factory)
        .with_hwnd(hwnd)
        .with_target_type(RenderTargetType::Default)
        .with_alpha_mode(AlphaMode::Unknown)
        .with_pixel_size(width, height)
        .build();
    if let Err(ref e) = res {
        println!("Error creating hwnd render target: {:?}", e);
    }
    res.map_err(|_| Error::D2Error)
}

/// Create a render target from a DXGI swapchain.
///
/// TODO: probably want to create a DeviceContext, it's more flexible.
pub(crate) unsafe fn create_render_target_dxgi(
    d2d_factory: &direct2d::Factory,
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
    pub fn d2d_factory(&self) -> &direct2d::Factory {
        self.d2d_factory
    }

    /// Return the raw Direct2D RenderTarget for this painting context. Note: it's possible
    /// this will be wrapped to make it easier to port.
    pub fn render_target(&mut self) -> &mut GenericRenderTarget {
        self.render_target
    }
}
