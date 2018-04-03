// Copyright 2017 Google Inc. All rights reserved.
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

use winapi::Interface;
use winapi::ctypes::{c_void};
use winapi::um::d2d1::*;
use winapi::um::d2d1_1::*;
use winapi::um::dcommon::*;
use winapi::um::winuser::*;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::windef::*;
use winapi::shared::winerror::*;

use direct2d;
use direct2d::render_target::{RenderTarget, RenderTargetBacking};

use Error;

/// Context for painting by app into window.
pub struct PaintCtx<'a> {
    pub(crate) d2d_factory: &'a direct2d::Factory,
    pub(crate) render_target: &'a mut RenderTarget,
}

struct HwndRtParams {
    hwnd: HWND,
    width: u32,
    height: u32,
}

unsafe impl RenderTargetBacking for HwndRtParams {
    fn create_target(self, factory: &mut ID2D1Factory1)
        -> Result<*mut ID2D1RenderTarget, HRESULT>
    {
        unsafe {
            let mut ptr: *mut ID2D1HwndRenderTarget = null_mut();
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                _type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_UNKNOWN,
                    alphaMode: D2D1_ALPHA_MODE_UNKNOWN,
                },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let hprops = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: D2D1_SIZE_U {
                    width: self.width,
                    height: self.height,
                },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            let hr = factory.CreateHwndRenderTarget(
                &props,
                &hprops,
                &mut ptr as *mut _,
            );

            if SUCCEEDED(hr) {
                Ok(ptr as *mut _)
            } else {
                Err(From::from(hr))
            }
        }
    }
}

pub(crate) unsafe fn create_render_target(d2d_factory: &direct2d::Factory, hwnd: HWND)
        -> Result<RenderTarget, Error>
{
    let mut rect: RECT = mem::uninitialized();
    GetClientRect(hwnd, &mut rect);
    let width = (rect.right - rect.left) as u32;
    let height = (rect.bottom - rect.top) as u32;
    let params = HwndRtParams { hwnd: hwnd, width: width, height: height };
    d2d_factory.create_render_target(params).map_err(|_| Error::D2Error)
}

struct DxgiBacking(*mut IDXGISurface);

unsafe impl RenderTargetBacking for DxgiBacking {
    fn create_target(self, factory: &mut ID2D1Factory1) -> Result<*mut ID2D1RenderTarget, HRESULT> {
        unsafe {
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                _type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: 192.0, // TODO: get this from window etc.
                dpiY: 192.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };

            let mut render_target: *mut ID2D1RenderTarget = null_mut();
            let res = factory.CreateDxgiSurfaceRenderTarget(self.0, &props, &mut render_target);
            if SUCCEEDED(res) {
                //(*render_target).SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
                Ok(render_target)
            } else {
                Err(res)
            }
        }
    }
}

pub(crate) unsafe fn create_render_target_dxgi(d2d_factory: &direct2d::Factory,
    swap_chain: *mut IDXGISwapChain1) -> Result<RenderTarget, Error>
{
    let mut buffer: *mut IDXGISurface = null_mut();
    let res = (*swap_chain).GetBuffer(0, &IDXGISurface::uuidof(),
        &mut buffer as *mut _ as *mut *mut c_void);
    let backing = DxgiBacking(buffer);
    let result = d2d_factory.create_render_target(backing);
    (*buffer).Release();
    result.map_err(|_| Error::D2Error)
}

impl<'a> PaintCtx<'a> {

    /// Return the raw Direct2D factory for this painting context. Note: it's possible
    /// this will be wrapped to make it easier to port.
    pub fn d2d_factory(&self) -> &direct2d::Factory {
        self.d2d_factory
    }

    /// Return the raw Direct2D RenderTarget for this painting context. Note: it's possible
    /// this will be wrapped to make it easier to port.
    pub fn render_target(&mut self) -> &mut RenderTarget {
        self.render_target
    }
}
