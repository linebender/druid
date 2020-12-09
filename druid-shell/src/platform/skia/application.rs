// Copyright 2020 The Druid Authors.
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

//! Web implementation of features at the application scope.

use std::rc::Rc;
use std::cell::RefCell;

use crate::application::AppHandler;

use super::clipboard::Clipboard;
use super::window::Window;
use anyhow::{anyhow, Context, Error};

use skulpin::CoordinateSystemHelper;
use skulpin::winit;
use skulpin::skia_safe;

use crate::piet::{Piet, PietText, RenderContext};

#[derive(Clone)]
pub(crate) struct Application {  
    /// The mutable `Application` state.
    state: Rc<RefCell<State>>,
}


/// The mutable `Application` state.
struct State {
    /// Whether `Application::quit` has already been called.
    quitting: bool,
    /// A collection of all the `Application` windows.
    window: Option<Rc<Window>>, // we only want to support one window for now
}

impl Application {
    pub fn new() -> Result<Application, Error> {
        let state = Rc::new(RefCell::new(State {
            quitting: false,
            window: None,
        }));
        Ok(Application{state})
    }

    pub fn add_window(&self, window: Rc<Window>) -> Result<(), Error> {
        borrow_mut!(self.state)?.window = Some(window);
        Ok(())
    }

    pub fn run(self, _handler: Option<Box<dyn AppHandler>>) {
        // Create the winit event loop
        let event_loop = winit::event_loop::EventLoop::<()>::with_user_event();
        // Set up the coordinate system to be fixed at 900x600, and use this as the default window size
        // This means the drawing code can be written as though the window is always 900x600. The
        // output will be automatically scaled so that it's always visible.
        let logical_size = winit::dpi::LogicalSize::new(900.0, 600.0);
        let visible_range = skulpin::skia_safe::Rect {
            left: 0.0,
            right: logical_size.width as f32,
            top: 0.0,
            bottom: logical_size.height as f32,
        };
        let scale_to_fit = skulpin::skia_safe::matrix::ScaleToFit::Center;
        // Create a single window
        let winit_window = winit::window::WindowBuilder::new()
            .with_title("Skulpin")
            .with_inner_size(logical_size)
            .build(&event_loop)
            .expect("Failed to create window");
    
        let window = skulpin::WinitWindow::new(&winit_window);
        // Create the renderer, which will draw to the window
        let renderer = skulpin::RendererBuilder::new()
            .use_vulkan_debug_layer(false)
            .coordinate_system(skulpin::CoordinateSystem::VisibleRange(
                visible_range,
                scale_to_fit,
            ))
            .build(&window);
    
        // Check if there were error setting up vulkan
        if let Err(e) = renderer {
            println!("Error during renderer construction: {:?}", e);
            panic!();
        }
    
        let mut renderer = renderer.unwrap();
    
        // Increment a frame count so we can render something that moves
        let mut frame_count = 0;
        event_loop.run(move |event, _window_target, control_flow| {
        
            let window = skulpin::WinitWindow::new(&winit_window);

            match event {
                //
                // Halt if the user requests to close the window
                //
                winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,

                //
                // Close if the escape key is hit
                //
                winit::event::Event::WindowEvent {
                    event:
                        winit::event::WindowEvent::KeyboardInput {
                            input:
                                winit::event::KeyboardInput {
                                    virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        },
                    ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,

                //
                // Request a redraw any time we finish processing events
                //
                winit::event::Event::MainEventsCleared => {
                    // Queue a RedrawRequested event.
                    winit_window.request_redraw();
                }

                //
                // Redraw
                //
                winit::event::Event::RedrawRequested(_window_id) => {
                    if let Err(e) = renderer.draw(&window, |canvas, coordinate_system_helper| {
                        let mut state = borrow_mut!(self.state).unwrap();
                        let main_window = state.window.as_mut().unwrap();
                        main_window.render(canvas);
                        //let main_window = borrow!(self.state).unwrap().window.unwrap();
                        //let mut state = borrow_mut!(self.state).unwrap();
                        //let main_window = state.window.as_mut().unwrap();
                        //let mut main_window = borrow_mut!(main_window).unwrap();
                        //let main_window = borrow!(self.state).unwrap().window.unwrap();
                        //main_window.render(&window, control_flow);
                        //draw(canvas, coordinate_system_helper, frame_count);
                        //frame_count += 1;
                    }) {
                        println!("Error during draw: {:?}", e);
                        *control_flow = winit::event_loop::ControlFlow::Exit
                    }
                }

                //
                // Ignore all other events
                //
                _ => {}
            }
        });
    }

    pub fn quit(&self) {}

    pub fn clipboard(&self) -> Clipboard {
        Clipboard
    }

    pub fn get_locale() -> String {
        //TODO ahem
        "en-US".into()
    }
}

