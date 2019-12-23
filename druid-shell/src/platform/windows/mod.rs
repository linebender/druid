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
use wio::com::ComPtr;
pub type HwndRenderTarget = ComPtr<winapi::um::d2d1::ID2D1HwndRenderTarget>;
pub type DeviceContext = ComPtr<winapi::um::d2d1_1::ID2D1DeviceContext>;
