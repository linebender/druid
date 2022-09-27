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

use std::any::Any;
use std::sync::{Arc, Mutex};

use druid_shell::kurbo::Size;

use druid_shell::{
    Application, Cursor, FileDialogOptions, FileDialogToken, FileInfo, FileSpec, HotKey, KeyEvent,
    Menu, MouseEvent, Region, SysMods, TimerToken, WinHandler, WindowBuilder, WindowHandle,
};
use piet_gpu_hal::{
    include_shader, BindType, Buffer, BufferUsage, ComputePassDescriptor, DescriptorSet, Image,
    ImageFormat, ImageLayout, Instance, InstanceFlags, Pipeline, Semaphore, Session, Swapchain,
};

#[derive(Default)]
struct HelloState {
    size: Size,
    handle: WindowHandle,
    gpu_state: Arc<Mutex<Option<GpuState>>>,
}

impl WinHandler for HelloState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn prepare_paint(&mut self) {
        self.handle.invalidate();
    }

    fn paint(&mut self, _: &Region) {
        unsafe {
            // TODO: wire up size
            let width = 1000;
            let height = 800;
            let mut state_guard = self.gpu_state.lock().unwrap();
            let state = state_guard.as_mut().unwrap();
            let frame_idx = state.current_frame % 2;
            let (image_idx, acquisition_semaphore) = state.swapchain.next().unwrap();
            let swap_image = state.swapchain.image(image_idx);

            // TODO: wire up time for animation purposes
            let i_time: f32 = 0.0;
            let config_data = [width, height, i_time.to_bits()];
            state.config_host.write(&config_data).unwrap();

            let mut cmd_buf = state.session.cmd_buf().unwrap();
            cmd_buf.begin();
            cmd_buf.image_barrier(&swap_image, ImageLayout::Undefined, ImageLayout::BlitDst);
            cmd_buf.copy_buffer(&state.config_host, &state.config_dev);
            cmd_buf.memory_barrier();

            cmd_buf.image_barrier(
                &state.staging_img,
                ImageLayout::Undefined,
                ImageLayout::General,
            );
            let wg_x = width / 16;
            let wg_y = height / 16;
            let mut pass = cmd_buf.begin_compute_pass(&ComputePassDescriptor::default());
            pass.dispatch(
                &state.pipeline,
                &state.descriptor_set,
                (wg_x, wg_y, 1),
                (16, 16, 1),
            );
            pass.end();
            cmd_buf.image_barrier(
                &state.staging_img,
                ImageLayout::General,
                ImageLayout::BlitSrc,
            );
            cmd_buf.blit_image(&state.staging_img, &swap_image);
            cmd_buf.image_barrier(&swap_image, ImageLayout::BlitDst, ImageLayout::Present);
            cmd_buf.finish();
            let submitted = state
                .session
                .run_cmd_buf(
                    cmd_buf,
                    &[&acquisition_semaphore],
                    &[&state.present_semaphores[frame_idx]],
                )
                .unwrap();
            state
                .swapchain
                .present(image_idx, &[&state.present_semaphores[frame_idx]])
                .unwrap();
            let start = std::time::Instant::now();
            submitted.wait().unwrap();
            println!("wait elapsed: {:?}", start.elapsed());
            state.current_frame += 1;
        }
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => {
                self.handle.close();
                Application::global().quit()
            }
            0x101 => {
                let options = FileDialogOptions::new().show_hidden().allowed_types(vec![
                    FileSpec::new("Rust Files", &["rs", "toml"]),
                    FileSpec::TEXT,
                    FileSpec::JPG,
                ]);
                self.handle.open_file(options);
            }
            0x102 => {
                let options = FileDialogOptions::new().show_hidden().allowed_types(vec![
                    FileSpec::new("Rust Files", &["rs", "toml"]),
                    FileSpec::TEXT,
                    FileSpec::JPG,
                ]);
                self.handle.save_as(options);
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn open_file(&mut self, _token: FileDialogToken, file_info: Option<FileInfo>) {
        println!("open file result: {:?}", file_info);
    }

    fn save_as(&mut self, _token: FileDialogToken, file: Option<FileInfo>) {
        println!("save file result: {:?}", file);
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        println!("keydown: {:?}", event);
        false
    }

    fn key_up(&mut self, event: KeyEvent) {
        println!("keyup: {:?}", event);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        println!("mouse_wheel {:?}", event);
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        self.handle.set_cursor(&Cursor::Arrow);
        println!("mouse_move {:?}", event);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        println!("mouse_down {:?}", event);
        self.render();
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        println!("mouse_up {:?}", event);
    }

    fn timer(&mut self, id: TimerToken) {
        println!("timer fired: {:?}", id);
    }

    fn size(&mut self, size: Size) {
        println!("size: {:?}", size);
        self.size = size;
    }

    fn got_focus(&mut self) {
        println!("Got focus");
    }

    fn lost_focus(&mut self) {
        println!("Lost focus");
    }

    fn request_close(&mut self) {
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl HelloState {
    fn render(&self) {
        unsafe {
            // TODO: wire up size
            let width = 1000;
            let height = 800;
            let mut state_guard = self.gpu_state.lock().unwrap();
            let state = state_guard.as_mut().unwrap();
            let frame_idx = state.current_frame % 2;
            let (image_idx, acquisition_semaphore) = state.swapchain.next().unwrap();
            let swap_image = state.swapchain.image(image_idx);

            // TODO: wire up time for animation purposes
            let i_time: f32 = 0.0;
            let config_data = [width, height, i_time.to_bits()];
            state.config_host.write(&config_data).unwrap();

            let mut cmd_buf = state.session.cmd_buf().unwrap();
            cmd_buf.begin();
            cmd_buf.image_barrier(&swap_image, ImageLayout::Undefined, ImageLayout::BlitDst);
            cmd_buf.copy_buffer(&state.config_host, &state.config_dev);
            cmd_buf.memory_barrier();

            cmd_buf.image_barrier(
                &state.staging_img,
                ImageLayout::Undefined,
                ImageLayout::General,
            );
            let wg_x = width / 16;
            let wg_y = height / 16;
            let mut pass = cmd_buf.begin_compute_pass(&ComputePassDescriptor::default());
            pass.dispatch(
                &state.pipeline,
                &state.descriptor_set,
                (wg_x, wg_y, 1),
                (16, 16, 1),
            );
            pass.end();
            cmd_buf.image_barrier(
                &state.staging_img,
                ImageLayout::General,
                ImageLayout::BlitSrc,
            );
            cmd_buf.blit_image(&state.staging_img, &swap_image);
            cmd_buf.image_barrier(&swap_image, ImageLayout::BlitDst, ImageLayout::Present);
            cmd_buf.finish();
            let submitted = state
                .session
                .run_cmd_buf(
                    cmd_buf,
                    &[&acquisition_semaphore],
                    &[&state.present_semaphores[frame_idx]],
                )
                .unwrap();
            state
                .swapchain
                .present(image_idx, &[&state.present_semaphores[frame_idx]])
                .unwrap();
            let start = std::time::Instant::now();
            submitted.wait().unwrap();
            println!("wait elapsed: {:?}", start.elapsed());
            state.current_frame += 1;
        }
    }
}

fn main() {
    tracing_subscriber::fmt().init();
    let mut file_menu = Menu::new();
    file_menu.add_item(
        0x100,
        "E&xit",
        Some(&HotKey::new(SysMods::Cmd, "q")),
        true,
        false,
    );
    file_menu.add_item(
        0x101,
        "O&pen",
        Some(&HotKey::new(SysMods::Cmd, "o")),
        true,
        false,
    );
    file_menu.add_item(
        0x102,
        "S&ave",
        Some(&HotKey::new(SysMods::Cmd, "s")),
        true,
        false,
    );
    let mut menubar = Menu::new();
    menubar.add_dropdown(Menu::new(), "Application", true);
    menubar.add_dropdown(file_menu, "&File", true);

    let app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(app.clone());
    let win_state = HelloState::default();
    let gpu_state = win_state.gpu_state.clone();
    builder.set_handler(Box::new(win_state));
    builder.set_title("Hello example");
    builder.set_menu(menubar);

    let window = builder.build().unwrap();
    unsafe {
        let width = 1000;
        let height = 800;
        let state = GpuState::new(&window, width, height).unwrap();
        *gpu_state.lock().unwrap() = Some(state);
    }
    window.show();

    app.run(None);
}

struct GpuState {
    current_frame: usize,
    instance: Instance,
    session: Session,
    swapchain: Swapchain,
    present_semaphores: Vec<Semaphore>,
    pipeline: Pipeline,
    descriptor_set: DescriptorSet,
    config_host: Buffer,
    config_dev: Buffer,
    staging_img: Image,
}

impl GpuState {
    unsafe fn new(
        window: &WindowHandle,
        width: usize,
        height: usize,
    ) -> Result<GpuState, Box<dyn std::error::Error>> {
        let instance = Instance::new(InstanceFlags::empty())?;
        let surface = instance.surface(&window)?;
        let device = instance.device()?;
        let swapchain = instance.swapchain(width, height, &device, &surface)?;
        let session = Session::new(device);
        let present_semaphores = (0..2)
            .map(|_| session.create_semaphore())
            .collect::<Result<Vec<_>, _>>()?;
        let shader_code = include_shader!(&session, "../shader/gen/shader");
        let pipeline =
            session.create_compute_pipeline(shader_code, &[BindType::Buffer, BindType::Image])?;
        let config_size = 12;
        let config_host =
            session.create_buffer(config_size, BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE)?;
        let config_dev =
            session.create_buffer(config_size, BufferUsage::COPY_DST | BufferUsage::STORAGE)?;
        let staging_img =
            session.create_image2d(width as u32, height as u32, ImageFormat::Rgba8)?;
        let descriptor_set = session
            .descriptor_set_builder()
            .add_buffers(&[&config_dev])
            .add_images(&[&staging_img])
            .build(&session, &pipeline)?;
        let current_frame = 0;
        Ok(GpuState {
            current_frame,
            instance,
            session,
            swapchain,
            present_semaphores,
            pipeline,
            descriptor_set,
            config_host,
            config_dev,
            staging_img,
        })
    }
}
