// Copyright 2017 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

//! Bureaucracy to create render targets for painting.
//!
//! Note that these are currently implemented using hwnd render targets
//! because they are are (relatively) easy, but for high performance we want
//! dxgi render targets so we can use present options for minimal
//! invalidation and low-latency frame timing.

use std::ptr::null_mut;

use winapi::ctypes::c_void;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::winerror::*;
use winapi::um::d2d1::*;
use winapi::um::dcommon::*;
use winapi::Interface;

use piet_common::d2d::D2DFactory;

use crate::backend::windows::DxgiSurfaceRenderTarget;
use crate::scale::Scale;

use super::error::Error;
use super::util::as_result;
use super::window::SCALE_TARGET_DPI;

/// Create a render target from a DXGI swapchain.
///
/// TODO: probably want to create a DeviceContext, it's more flexible.
pub(crate) unsafe fn create_render_target_dxgi(
    d2d_factory: &D2DFactory,
    swap_chain: *mut IDXGISwapChain1,
    scale: Scale,
    transparent: bool,
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
            alphaMode: if transparent {
                D2D1_ALPHA_MODE_PREMULTIPLIED
            } else {
                D2D1_ALPHA_MODE_IGNORE
            },
        },
        dpiX: (scale.x() * SCALE_TARGET_DPI) as f32,
        dpiY: (scale.y() * SCALE_TARGET_DPI) as f32,
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
