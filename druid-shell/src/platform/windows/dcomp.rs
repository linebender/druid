// Copyright 2018 The Druid Authors.
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

//! Safe-ish wrappers for DirectComposition and related interfaces.

// This module could become a general wrapper for DirectComposition, but
// for now we're just using what we need to get a swapchain up.
#![allow(unused)]

use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr::{null, null_mut};

use log::error;

use winapi::shared::dxgi::IDXGIDevice;
use winapi::shared::dxgi1_2::DXGI_ALPHA_MODE_IGNORE;
use winapi::shared::dxgiformat::DXGI_FORMAT_B8G8R8A8_UNORM;
use winapi::shared::minwindef::{FALSE, TRUE};
use winapi::shared::windef::{HWND, POINT, RECT};
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::d2d1::*;
use winapi::um::d2d1_1::*;
use winapi::um::d3d11::*;
use winapi::um::d3dcommon::D3D_DRIVER_TYPE_HARDWARE;
use winapi::um::dcomp::*;
use winapi::um::dcompanimation::*;
use winapi::um::unknwnbase::IUnknown;
use winapi::um::winnt::HRESULT;
use winapi::Interface;
use wio::com::ComPtr;

use super::util::OPTIONAL_FUNCTIONS;

unsafe fn wrap<T, U, F>(hr: HRESULT, ptr: *mut T, f: F) -> Result<U, HRESULT>
where
    F: Fn(ComPtr<T>) -> U,
    T: Interface,
{
    if SUCCEEDED(hr) {
        Ok(f(ComPtr::from_raw(ptr)))
    } else {
        Err(hr)
    }
}

fn unit_err(hr: HRESULT) -> Result<(), HRESULT> {
    if SUCCEEDED(hr) {
        Ok(())
    } else {
        Err(hr)
    }
}

pub struct D3D11Device(ComPtr<ID3D11Device>);
pub struct D2D1Device(ComPtr<ID2D1Device>);
pub struct DCompositionDevice(ComPtr<IDCompositionDevice>);
pub struct DCompositionTarget(ComPtr<IDCompositionTarget>);
pub struct DCompositionVisual(ComPtr<IDCompositionVisual>);
pub struct DCompositionVirtualSurface(ComPtr<IDCompositionVirtualSurface>);

/// A trait for content which can be added to a visual.
pub(crate) trait Content {
    unsafe fn unknown_ptr(&mut self) -> *mut IUnknown;
}

impl D3D11Device {
    /// Creates a new device with basic defaults.
    pub(crate) fn new_simple() -> Result<D3D11Device, HRESULT> {
        unsafe {
            let mut d3d11_device: *mut ID3D11Device = null_mut();
            let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT; // could probably set single threaded
            let hr = D3D11CreateDevice(
                null_mut(),
                D3D_DRIVER_TYPE_HARDWARE,
                null_mut(),
                flags,
                null(),
                0,
                D3D11_SDK_VERSION,
                &mut d3d11_device,
                null_mut(),
                null_mut(),
            );
            if !SUCCEEDED(hr) {
                error!("D3D11CreateDevice: 0x{:x}", hr);
            }
            wrap(hr, d3d11_device, D3D11Device)
        }
    }

    pub(crate) fn create_d2d1_device(&mut self) -> Result<D2D1Device, HRESULT> {
        unsafe {
            let mut dxgi_device: ComPtr<IDXGIDevice> = self.0.cast()?;
            let mut d2d1_device: *mut ID2D1Device = null_mut();
            let hr = D2D1CreateDevice(dxgi_device.as_raw(), null(), &mut d2d1_device);
            wrap(hr, d2d1_device, D2D1Device)
        }
    }

    pub(crate) fn raw_ptr(&mut self) -> *mut ID3D11Device {
        self.0.as_raw()
    }
}

impl D2D1Device {
    /// Create a wrapped DCompositionDevice object. Note: returns Err(0) on systems
    /// not supporting DirectComposition, available 8.1 and above.
    pub(crate) fn create_composition_device(&mut self) -> Result<DCompositionDevice, HRESULT> {
        unsafe {
            let create = OPTIONAL_FUNCTIONS.DCompositionCreateDevice2.ok_or(0)?;
            let mut dcomp_device: *mut IDCompositionDevice = null_mut();
            let hr = create(
                self.0.as_raw() as *mut IUnknown,
                &IDCompositionDevice::uuidof(),
                &mut dcomp_device as *mut _ as *mut _,
            );
            wrap(hr, dcomp_device, DCompositionDevice)
        }
    }
}

impl DCompositionDevice {
    pub(crate) unsafe fn create_target_for_hwnd(
        &mut self,
        hwnd: HWND,
        topmost: bool,
    ) -> Result<DCompositionTarget, HRESULT> {
        let mut dcomp_target: *mut IDCompositionTarget = null_mut();
        let hr =
            self.0
                .CreateTargetForHwnd(hwnd, if topmost { TRUE } else { FALSE }, &mut dcomp_target);
        wrap(hr, dcomp_target, DCompositionTarget)
    }

    pub(crate) fn create_visual(&mut self) -> Result<DCompositionVisual, HRESULT> {
        unsafe {
            let mut visual: *mut IDCompositionVisual = null_mut();
            let hr = self.0.CreateVisual(&mut visual);
            wrap(hr, visual, DCompositionVisual)
        }
    }

    /// Creates an RGB surface. Probably should allow more options (including alpha).
    pub(crate) fn create_virtual_surface(
        &mut self,
        height: u32,
        width: u32,
    ) -> Result<DCompositionVirtualSurface, HRESULT> {
        unsafe {
            let mut surface: *mut IDCompositionVirtualSurface = null_mut();
            let hr = self.0.CreateVirtualSurface(
                width,
                height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_IGNORE,
                &mut surface,
            );
            wrap(hr, surface, DCompositionVirtualSurface)
        }
    }

    pub(crate) fn commit(&mut self) -> Result<(), HRESULT> {
        unsafe { unit_err(self.0.Commit()) }
    }
}

impl DCompositionTarget {
    // alternatively could be set_root with an option
    pub(crate) fn clear_root(&mut self) -> Result<(), HRESULT> {
        unsafe { unit_err(self.0.SetRoot(null_mut())) }
    }

    pub(crate) fn set_root(&mut self, visual: &mut DCompositionVisual) -> Result<(), HRESULT> {
        unsafe { unit_err(self.0.SetRoot(visual.0.as_raw())) }
    }
}

impl DCompositionVisual {
    pub(crate) fn set_content<T: Content>(&mut self, content: &mut T) -> Result<(), HRESULT> {
        unsafe { self.set_content_raw(content.unknown_ptr()) }
    }

    // TODO: impl Content trait for swapchain, for type safety
    pub(crate) unsafe fn set_content_raw(&mut self, content: *mut IUnknown) -> Result<(), HRESULT> {
        unit_err(self.0.SetContent(content))
    }

    pub(crate) fn set_pos(&mut self, x: f32, y: f32) {
        unsafe {
            self.0.SetOffsetX_1(x);
            self.0.SetOffsetY_1(y);
        }
    }
}

// We don't actually need to draw into DirectComposition virtual surfaces now, this is
// experimental and based on an older version of direct2d-rs. Probably delete.

/*
struct DcBacking(*mut ID2D1DeviceContext);
unsafe impl RenderTargetBacking for DcBacking {
    fn create_target(self, _factory: &mut ID2D1Factory1) -> Result<*mut ID2D1RenderTarget, HRESULT> {
        Ok(self.0 as *mut ID2D1RenderTarget)
    }
}

// TODO: support common methods with DCompositionSurface, probably should be trait
impl DCompositionVirtualSurface {
    // could try to expose more DeviceContext capability
    pub fn begin_draw(&mut self, d2d_factory: &direct2d::Factory, rect: Option<RECT>)
        -> Result<RenderTarget, HRESULT>
    {
        unsafe {
            let mut dc: *mut ID2D1DeviceContext = null_mut();
            let rect_ptr = match rect {
                None => null(),
                Some(r) => &r,
            };
            let mut offset: POINT = mem::uninitialized();
            let hr = self.0.BeginDraw(rect_ptr, &ID2D1DeviceContext::uuidof(),
                &mut dc as *mut _ as *mut _, &mut offset);
            if !SUCCEEDED(hr) {
                return Err(hr);
            }
            let backing = DcBacking(dc);
            let mut rt = d2d_factory.create_render_target(backing).map_err(|e|
                match e {
                    direct2d::Error::ComError(hr) => hr,
                    _ => 0,
                })?;
            // TODO: either move dpi scaling somewhere else or figure out how to
            // set it correctly here.
            rt.set_transform(&Matrix3x2F::new([[2.0, 0.0], [0.0, 2.0],
                [offset.x as f32, offset.y as f32]]));
            Ok(rt)
        }
    }

    pub fn end_draw(&mut self) -> Result<(), HRESULT> {
        unsafe {
            unit_err(self.0.EndDraw())
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), HRESULT> {
        unsafe {
            unit_err(self.0.Resize(width, height))
        }
    }
}

impl Content for DCompositionVirtualSurface {
    unsafe fn unknown_ptr(&mut self) -> *mut IUnknown {
        self.0.as_raw() as *mut IUnknown
    }
}

*/
