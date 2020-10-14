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

use std::ptr::{null, null_mut};

use log::error;

use winapi::shared::winerror::SUCCEEDED;
use winapi::um::d3d11::*;
use winapi::um::d3dcommon::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use winapi::um::winnt::HRESULT;
use winapi::Interface;
use wio::com::ComPtr;

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

pub struct D3D11Device(ComPtr<ID3D11Device>);

impl D3D11Device {
    /// Creates a new device with basic defaults.
    pub(crate) fn new_simple() -> Result<D3D11Device, HRESULT> {
        let mut hr = 0;
        unsafe {
            let mut d3d11_device: *mut ID3D11Device = null_mut();
            // Note: could probably set single threaded in flags for small performance boost.
            let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
            // Prefer hardware but use warp if it's the only driver available.
            for driver_type in &[D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
                hr = D3D11CreateDevice(
                    null_mut(),
                    *driver_type,
                    null_mut(),
                    flags,
                    null(),
                    0,
                    D3D11_SDK_VERSION,
                    &mut d3d11_device,
                    null_mut(),
                    null_mut(),
                );
                if SUCCEEDED(hr) {
                    break;
                }
            }
            if !SUCCEEDED(hr) {
                error!("D3D11CreateDevice: 0x{:x}", hr);
            }
            wrap(hr, d3d11_device, D3D11Device)
        }
    }

    pub(crate) fn raw_ptr(&mut self) -> *mut ID3D11Device {
        self.0.as_raw()
    }
}
