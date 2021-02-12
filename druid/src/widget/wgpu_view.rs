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

//! A view hosting a wgpu instance for 3D rendering.
//!
//! This widget has several limitations compared to other standard widget due
//! to the way it interfaces with wgpu via its own native window, instead of
//! reusing the same top-level native window like other widgets do. In particular
//! this means it can only paint within its bounds and cannot be painted over by
//! any other widget, and therefore does not support being embedded into other
//! widgets like scroll views which partially obscure a widget's client area.

#![cfg(feature = "wgpu_view")]

use crate::widget::prelude::*;
use crate::{Data, NativeWindowHandle};

use log::{debug, info};

/// A trait for rendering custom content into a `WgpuView` by accessing its internal wgpu pipeline.
pub trait WgpuRenderer {
    /// Callback invoked to initilize the painter once the wgpu device and swap chain are up.
    fn init(
        &mut self,
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    /// Callback invoked when the wgpu view is resized.
    ///
    /// This is typically used to update any camera projection matrix, and/or recreate any render
    /// target. This is invoked before the new swap chain with the new window size is created, but
    /// after the size has been updated in `sc_desc`.
    fn resize(
        &mut self,
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    /// Callback invoked when the wgpu view needs to be repaint.
    ///
    /// This is where rendering must be submitted by the painter to the given queue.
    fn render(
        &mut self,
        frame: &wgpu::SwapChainTexture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
}

/// Data for a `WgpuView` lazily created once the native window is available.
#[allow(dead_code)]
struct RenderData {
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pool: futures::executor::LocalPool,
    spawner: futures::executor::LocalSpawner,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
}

/// A 3D view widget containing a wgpu instance.
pub struct WgpuView {
    /// The painter provided by the user to render into the view.
    renderer: Box<dyn WgpuRenderer>,
    /// Handle to the native window.
    native_window: Option<NativeWindowHandle>,
    /// The render data lazily created once the native window is available.
    /// This is `None` until the widget receives the `NativeWindowConnected`
    /// event, and is a valid data after that until the widget is destroyed,
    /// unless an error occurs.
    render_data: Option<RenderData>,
}

impl WgpuView {
    /// Create a new view which uses the given renderer to render its content.
    pub fn new(renderer: impl WgpuRenderer + 'static) -> Self {
        WgpuView {
            renderer: Box::new(renderer),
            native_window: None, // Filled when NativeWindowConnected is received
            render_data: None,   // Filled by create_render_data() later
        }
    }

    #[allow(unsafe_code)]
    fn create_render_data(&mut self) {
        let backend = wgpu::BackendBit::DX11;
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface(&self.native_window.as_ref().unwrap().0) };
        debug!("Created WGPU surface with {:?} backend", backend);
        let power_preference = wgpu::PowerPreference::HighPerformance;
        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: Some(&surface),
            }))
            .unwrap();
        debug!("Created WGPU adapter {:?}", adapter);
        let adapter_info = adapter.get_info();
        info!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
        let adapter_features = adapter.features();
        let needed_limits = wgpu::Limits::default();
        debug!("Adapter features: {:?}", adapter_features);
        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: adapter_features,
                limits: needed_limits,
                shader_validation: true,
            },
            None,
        ))
        .unwrap();
        debug!("Created WGPU device {:?} and queue {:?}", device, queue);

        let (pool, spawner) = {
            let local_pool = futures::executor::LocalPool::new();
            let spawner = local_pool.spawner();
            (local_pool, spawner)
        };
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: 256,
            height: 256,
            present_mode: wgpu::PresentMode::Fifo, //Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        debug!("Swap-chain created: {:?}", swap_chain);

        self.renderer.init(&sc_desc, &device, &queue);

        self.render_data = Some(RenderData {
            instance,
            surface,
            adapter,
            device,
            queue,
            pool,
            spawner,
            sc_desc,
            swap_chain,
        });
    }
}

impl<T: Data> Widget<T> for WgpuView {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            Event::NativeWindowConnected(native_window) => {
                self.native_window = Some(native_window.clone());
                self.create_render_data();
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &T, _env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            ctx.request_native_window(Size::new(160.0, 120.0));
        } else if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        } else if let LifeCycle::Size(size) = event {
            if let Some(render_data) = &mut self.render_data {
                render_data.sc_desc.width = std::cmp::max(1, size.width as u32);
                render_data.sc_desc.height = std::cmp::max(1, size.height as u32);
                self.renderer.resize(
                    &render_data.sc_desc,
                    &render_data.device,
                    &render_data.queue,
                );
                render_data.swap_chain = render_data
                    .device
                    .create_swap_chain(&render_data.surface, &render_data.sc_desc);
            }
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, _env: &Env) -> Size {
        bc.max()
    }

    fn post_render(&mut self) {
        if let Some(render_data) = &mut self.render_data {
            let frame = match render_data.swap_chain.get_current_frame() {
                Ok(frame) => frame,
                Err(_) => {
                    render_data.swap_chain = render_data
                        .device
                        .create_swap_chain(&render_data.surface, &render_data.sc_desc);
                    render_data
                        .swap_chain
                        .get_current_frame()
                        .expect("Failed to acquire next swap chain texture!")
                }
            };
            let device: &wgpu::Device = &render_data.device;
            let queue: &wgpu::Queue = &render_data.queue;
            self.renderer.render(&frame.output, device, queue);
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, _env: &Env) {
        // This widget does paint anything with the built-in Piet-based drawing pipeline.
        // All rendering is done externally via wgpu in post_render(), after the Piet-based
        // drawing has occurred.
        if let Some(native_window) = &self.native_window {
            native_window
                .0
                .set_native_layout(Some(ctx.native_origin), Some(ctx.widget_state.size()));
        }
    }
}
