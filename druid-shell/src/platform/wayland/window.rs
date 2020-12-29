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

#![allow(clippy::single_match)]

use anyhow::{anyhow, format_err};
use cairo::Surface;
use nix::{
    errno::Errno,
    fcntl::OFlag,
    sys::{
        mman::{mmap, munmap, shm_open, MapFlags, ProtFlags},
        stat::Mode,
    },
    unistd::{close, ftruncate},
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashSet, VecDeque},
    convert::{TryFrom, TryInto},
    ffi::c_void,
    fmt,
    ops::Deref,
    os::{
        raw::{c_int, c_uint},
        unix::io::RawFd,
    },
    panic::Location,
    ptr::{self, NonNull},
    rc::{Rc, Weak as WeakRc},
    slice,
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant, SystemTime},
};
use wayland_client::{
    self as wl,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_callback,
        wl_keyboard::{self, WlKeyboard},
        wl_output::WlOutput,
        wl_pointer::{self, WlPointer},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::{self, WlSurface},
    },
};
use wayland_cursor::CursorImageBuffer;
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_toplevel_decoration_v1::{
        Event as ZxdgToplevelDecorationV1Event, Mode as DecorationMode, ZxdgToplevelDecorationV1,
    },
    xdg_shell::client::{
        xdg_surface::{Event as XdgSurfaceEvent, XdgSurface},
        xdg_toplevel::{Event as XdgTopLevelEvent, XdgToplevel},
        xdg_wm_base::XdgWmBase,
    },
};

use super::{
    application::{Application, ApplicationData, Output},
    buffer::{Buffer, Buffers, Mmap, RawRect, RawSize, Shm},
    dialog, keycodes,
    menu::Menu,
    pointer::{MouseEvtKind, Pointer},
    util, Changed, NUM_FRAMES, PIXEL_WIDTH,
};
use crate::{
    common_util::{ClickCounter, IdleCallback},
    dialog::{FileDialogOptions, FileDialogType, FileInfo},
    error::Error as ShellError,
    keyboard::{KbKey, KeyEvent, KeyState, Modifiers},
    kurbo::{Insets, Point, Rect, Size, Vec2},
    mouse::{Cursor, CursorDesc, MouseButton, MouseButtons, MouseEvent},
    piet::ImageFormat,
    piet::{Piet, PietText, RenderContext},
    platform::shared::Timer,
    region::Region,
    scale::{Scalable, Scale, ScaledArea},
    text::Event,
    window::{self, FileDialogToken, IdleToken, TimerToken, WinHandler, WindowLevel},
    TextFieldToken,
};

//TODO we flash the old window size before adjusting to the new size. This seems to leave some
//artifact on another monitor if the image would have spread across. The todo is investigate if we
//can avoid this. I think it might be something to do with telling the compositor about our new
//geometry?
//
// I think this bit of debug output might be useful (from GTK, which doesn't suffer from this
// problem). UPDATE I'm using the unstable window decoration protocol. GTK doesn't use this, since
// there is some arcane thing to do with `set_opaque_region` to make it backwards compatible. Let's
// ignore this for now.
//[2566306.298] xdg_toplevel@32.configure(580, 450, array)
//[2566306.344] xdg_surface@22.configure(74094)
//[2566306.359]  -> wl_buffer@36.destroy()
//[2566306.366]  -> wl_shm_pool@37.destroy()
//[2566306.728]  -> wl_surface@26.set_buffer_scale(2)
//[2566306.752]  -> xdg_surface@22.ack_configure(74094)
//[2566307.207]  -> wl_shm@4.create_pool(new id wl_shm_pool@40, fd 31, 4176000)
//[2566307.229]  -> wl_shm_pool@40.create_buffer(new id wl_buffer@41, 0, 1160, 900, 4640, 0)
//[2566315.088]  -> wl_surface@26.attach(wl_buffer@41, 0, 0)
//[2566315.111]  -> wl_surface@26.set_buffer_scale(2)
//[2566315.116]  -> wl_surface@26.damage(0, 0, 580, 450)
//[2566315.137]  -> xdg_toplevel@32.set_min_size(400, 427)
//[2566315.153]  -> xdg_toplevel@32.set_max_size(0, 0)
//[2566315.160]  -> xdg_surface@22.set_window_geometry(0, 0, 580, 450)
//[2566315.170]  -> wl_compositor@6.create_region(new id wl_region@42)
//[2566315.176]  -> wl_region@42.add(0, 0, 580, 450)
//[2566315.190]  -> wl_surface@26.set_opaque_region(wl_region@42)
//[2566315.195]  -> wl_region@42.destroy()
//[2566315.237]  -> wl_surface@26.frame(new id wl_callback@43)
//[2566315.256]  -> wl_surface@26.commit()

// In cairo and Wayland, alpha is pre-multiplied. Yay.

#[derive(Default, Clone)]
pub struct WindowHandle {
    // holding a weak reference to the window is copied from the windows backend.
    pub(crate) data: WeakRc<WindowData>,
}

impl WindowHandle {
    pub fn show(&self) {}

    pub fn resizable(&self, resizable: bool) {
        //todo!()
    }

    pub fn show_titlebar(&self, show_titlebar: bool) {
        //todo!()
    }

    pub fn set_position(&self, position: Point) {
        //todo!()
    }

    pub fn get_position(&self) -> Point {
        todo!()
    }

    pub fn content_insets(&self) -> Insets {
        // TODO
        Insets::from(0.)
    }

    pub fn set_level(&self, level: WindowLevel) {
        log::warn!("level is unsupported on wayland");
    }

    pub fn set_size(&self, size: Size) {
        log::warn!("setting the size dynamically is unsupported on wayland");
    }

    pub fn get_size(&self) -> Size {
        if let Some(data) = self.data.upgrade() {
            // size in pixels, so we must apply scale
            // TODO check the logic here.
            let logical_size = data.logical_size.get();
            let scale = data.scale.get() as f64;
            Size::new(
                logical_size.width as f64 * scale,
                logical_size.height as f64 * scale,
            )
        } else {
            // TODO panic?
            Size::ZERO
        }
    }

    pub fn set_window_state(&mut self, size_state: window::WindowState) {
        //todo!()
    }

    pub fn get_window_state(&self) -> window::WindowState {
        todo!()
    }

    pub fn handle_titlebar(&self, _val: bool) {
        todo!()
    }

    /// Close the window.
    pub fn close(&self) {
        if let Some(data) = self.data.upgrade() {
            // TODO destroy resources
            if let Some(app_data) = data.app_data.upgrade() {
                app_data.shutdown.set(true);
            }
        }
    }

    /// Bring this window to the front of the window stack and give it focus.
    pub fn bring_to_front_and_focus(&self) {
        //todo!()
    }

    /// Request a new paint, but without invalidating anything.
    pub fn request_anim_frame(&self) {
        if let Some(data) = self.data.upgrade() {
            if !data.anim_frame_requested.get() {
                let cb = data.wl_surface.frame();
                let handle = self.clone();
                cb.quick_assign(with_cloned!(data; move |_, event, _| match event {
                    wl_callback::Event::Done { callback_data } => {
                        data.anim_frame_requested.set(false);
                        data.request_paint();
                    }
                    _ => panic!("done is the only event"),
                }));
                data.anim_frame_requested.set(true);
            }
        }
    }

    /// Request invalidation of the entire window contents.
    pub fn invalidate(&self) {
        // This is one of 2 methods the user can use to schedule a repaint, the other is
        // `invalidate_rect`.
        if let Some(data) = self.data.upgrade() {
            data.invalidate()
        }
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    pub fn invalidate_rect(&self, rect: Rect) {
        if let Some(data) = self.data.upgrade() {
            data.invalidate_rect(rect)
        }
    }

    pub fn text(&self) -> PietText {
        PietText::new()
    }

    pub fn add_text_field(&self) -> TextFieldToken {
        todo!()
    }

    pub fn remove_text_field(&self, token: TextFieldToken) {
        todo!()
    }

    pub fn set_focused_text_field(&self, active_field: Option<TextFieldToken>) {
        todo!()
    }

    pub fn update_text_field(&self, token: TextFieldToken, update: Event) {
        todo!()
    }

    pub fn request_timer(&self, deadline: Instant) -> TimerToken {
        if let Some(data) = self.data.upgrade() {
            if let Some(app_data) = data.app_data.upgrade() {
                //println!("Timer requested");
                let now = instant::Instant::now();
                let mut timers = app_data.timers.borrow_mut();
                let sooner = timers
                    .peek()
                    .map(|timer| deadline < timer.deadline())
                    .unwrap_or(true);
                let timer = Timer::new(deadline, data.id());
                timers.push(timer);
                // It is possible that the deadline has passed since it was set.
                // TODO replace `Duration::new(0, 0)` with `Duration::ZERO` when it is stable.
                let timeout = if deadline < now {
                    Duration::new(0, 0)
                } else {
                    deadline - now
                };
                if sooner {
                    app_data.timer_handle.cancel_all_timeouts();
                    app_data.timer_handle.add_timeout(timeout, timer.token());
                }
                return timer.token();
            }
        }
        panic!("requested timer on a window that was destroyed");
    }

    pub fn set_cursor(&mut self, cursor: &Cursor) {
        if let Some(data) = self.data.upgrade() {
            let mut _pointer = data.pointer.borrow_mut();
            let pointer = _pointer.as_mut().unwrap();
            // Setting a new cursor involves communicating with the server, so don't do it if we
            // don't have to.
            if matches!(&pointer.current_cursor, Some(c) if c == cursor) {
                return;
            }
            pointer.current_cursor = Some(cursor.clone());
        }
    }

    pub fn make_cursor(&self, desc: &CursorDesc) -> Option<Cursor> {
        todo!()
    }

    pub fn open_file(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        todo!()
    }

    pub fn save_as(&mut self, options: FileDialogOptions) -> Option<FileDialogToken> {
        todo!()
    }

    /// Get a handle that can be used to schedule an idle task.
    pub fn get_idle_handle(&self) -> Option<IdleHandle> {
        None
    }

    /// Get the `Scale` of the window.
    pub fn get_scale(&self) -> Result<Scale, ShellError> {
        if let Some(data) = self.data.upgrade() {
            let scale = data.scale.get() as f64;
            Ok(Scale::new(scale, scale))
        } else {
            Err(ShellError::WindowDropped)
        }
    }

    pub fn set_menu(&self, menu: Menu) {
        todo!()
    }

    pub fn show_context_menu(&self, menu: Menu, _pos: Point) {
        todo!()
    }

    pub fn set_title(&self, title: impl Into<String>) {
        if let Some(data) = self.data.upgrade() {
            data.xdg_toplevel.set_title(title.into());
        }
    }
}

pub struct WindowData {
    pub(crate) app_data: WeakRc<ApplicationData>,
    pub(crate) wl_surface: wl::Main<WlSurface>,
    pub(crate) xdg_surface: wl::Main<XdgSurface>,
    pub(crate) xdg_toplevel: wl::Main<XdgToplevel>,
    pub(crate) zxdg_toplevel_decoration_v1: wl::Main<ZxdgToplevelDecorationV1>,
    /// The outputs that our surface is present on (we should get the first enter event early).
    pub(crate) outputs: RefCell<HashSet<u32>>,
    /// Buffers in our shared memory.
    // Buffers sometimes need to move references to themselves into closures, so must be behind a
    // reference counter.
    pub(crate) buffers: Rc<Buffers<{ NUM_FRAMES as usize }>>,
    /// The logical size of the next frame.
    pub(crate) logical_size: Cell<Size>,
    /// The scale we are rendering to (defaults to 1)
    pub(crate) scale: Cell<i32>,
    /// Whether we've currently got keyboard focus.
    pub(crate) keyboard_focus: Cell<bool>,
    /// If we've currently got pointer focus, this object tracks pointer state.
    pub(crate) pointer: RefCell<Option<Pointer>>,
    /// Whether we have requested an animation frame. This stops us requesting more than 1.
    anim_frame_requested: Cell<bool>,
    /// Track whether an event handler invalidated any regions. After the event handler has been
    /// released, repaint if true. TODO refactor this into an enum of evens that might call in to
    /// user code, and so need to be deferred.
    paint_scheduled: Cell<bool>,
    /// Contains the callbacks from user code.
    pub(crate) handler: RefCell<Box<dyn WinHandler>>,
    /// Rects of the image that are damaged and need repainting in the logical coordinate space.
    ///
    /// This lives outside `data` because they can be borrowed concurrently without re-entrancy.
    damaged_region: RefCell<Region>,
}

impl WindowData {
    /// Sets the physical size.
    ///
    /// Up to the caller to make sure `buffers.size`, `logical_size` and `scale` are consistent.
    fn set_physical_size(&self, new_size: RawSize) {
        self.buffers.set_size(new_size)
    }

    /// Assert that the physical size = logical size * scale
    #[allow(unused)]
    fn assert_size(&self) {
        assert_eq!(
            self.buffers.size(),
            RawSize::from(self.logical_size.get()).scale(self.scale.get()),
            "phy {:?} == logic {:?} * {}",
            self.buffers.size(),
            self.logical_size.get(),
            self.scale.get()
        );
    }

    /// Recompute the scale to use (the maximum of all the scales for the different outputs this
    /// screen is drawn to).
    fn recompute_scale(&self) -> i32 {
        if let Some(app_data) = self.app_data.upgrade() {
            let mut scale = 0;
            for id in self.outputs.borrow().iter() {
                if let Some(output) = app_data.outputs.borrow().get(&id) {
                    scale = scale.max(output.scale);
                } else {
                    log::warn!(
                        "we still have a reference to an output that's gone away. The output had id {}",
                        id
                    );
                }
            }
            if scale == 0 {
                log::warn!("wayland never reported which output we are drawing to");
                1
            } else {
                scale
            }
        } else {
            panic!("should never recompute scale of window that has been dropped");
        }
    }

    fn set_cursor(&self, buf: &CursorImageBuffer) {
        let (hotspot_x, hotspot_y) = buf.hotspot();
        let _pointer = self.pointer.borrow();
        let pointer = _pointer.as_ref().unwrap();
        pointer.wl_pointer.set_cursor(
            pointer.enter_serial,
            Some(&pointer.cursor_surface),
            hotspot_x as i32,
            hotspot_y as i32,
        );
        pointer.cursor_surface.attach(Some(&*buf), 0, 0);
        pointer
            .cursor_surface
            .damage_buffer(0, 0, i32::MAX, i32::MAX);
        pointer.cursor_surface.commit();
    }

    /// Get the wayland object id for the `wl_surface` associated with this window.
    ///
    /// We use this as the key for the window.
    pub(crate) fn id(&self) -> u32 {
        wl::Proxy::from(self.wl_surface.detach()).id()
    }

    /// Sets the scale
    ///
    /// Up to the caller to make sure `physical_size`, `logical_size` and `scale` are consistent.
    fn set_scale(&self, new_scale: i32) -> Changed {
        if self.scale.get() != new_scale {
            self.scale.set(new_scale);
            // (re-entrancy) Report change to client
            let druid_scale = Scale::new(new_scale as f64, new_scale as f64);
            self.handler.borrow_mut().scale(druid_scale);
            Changed::Changed
        } else {
            Changed::Unchanged
        }
    }

    /// Schedule a paint (in response to invalidation).
    pub(crate) fn schedule_paint(&self) {
        self.paint_scheduled.set(true);
    }

    /// If a repaint was scheduled, then execute it.
    pub(crate) fn check_for_scheduled_paint(&self) {
        if self.paint_scheduled.get() {
            self.request_paint();
        }
    }

    /// Request to `buffers` that the next frame be painted.
    ///
    /// If the next frame is ready, then it will be painted immediately, otherwise a paint will be
    /// scheduled to take place when a frame is released.
    ///
    /// ```text
    /// self.request_paint -> calls buffers.request_paint -> calls self.paint (possibly not immediately)
    /// ```
    fn request_paint(&self) {
        self.paint_scheduled.set(false);
        self.buffers.request_paint();
    }

    /// Paint the next frame.
    ///
    /// The buffers object is responsible for calling this function after we called
    /// `request_paint`.
    ///
    /// - `buf` is what we draw the frame into
    /// - `size` is the physical size in pixels we are drawing.
    /// - `force` means draw the whole frame, even if it wasn't all invalidated.
    pub(crate) fn paint(&self, size: RawSize, buf: &mut [u8], force: bool) {
        //log::trace!("Paint call");
        //self.data.borrow().assert_size();
        if force {
            self.invalidate();
        } else {
            let mut damaged_region = self.damaged_region.borrow_mut();
            for rect in damaged_region.rects() {
                // Convert it to physical coordinate space.
                let rect = RawRect::from(*rect).scale(self.scale.get());
                self.wl_surface.damage_buffer(
                    rect.x0,
                    rect.y0,
                    rect.x1 - rect.x0,
                    rect.y1 - rect.y0,
                );
            }
            if damaged_region.is_empty() {
                // Nothing to draw, so we can finish here!
                return;
            }
        }

        // create cairo context (safety: we must drop the buffer before we commit the frame)
        // TODO: Cairo is native-endian while wayland is little-endian, which is a pain. Currently
        // will give incorrect results on big-endian architectures.
        // TODO cairo might use a different stride than the width of the format. Since we always
        // use argb32 which is 32-bit aligned we should be ok, but strictly speaking cairo might
        // choose a wider stride and read past the end of our buffer (UB). Fixing this would
        // require a fair bit of effort.
        unsafe {
            let physical_size = self.buffers.size();
            // We're going to lie about the lifetime of our buffer here. This is (I think) ok,
            // becuase the Rust wrapper for cairo is overly pessimistic: the buffer only has to
            // last as long as the `ImageSurface` (which we know this buffer will).
            let buf: &'static mut [u8] = &mut *(buf as *mut _);
            let cairo_surface = cairo::ImageSurface::create_for_data(
                buf,
                cairo::Format::ARgb32,
                physical_size.width,
                physical_size.height,
                physical_size.width * PIXEL_WIDTH,
            )
            .unwrap();
            let ctx = cairo::Context::new(&cairo_surface);
            // Apply scaling
            let scale = self.scale.get() as f64;
            ctx.scale(scale, scale);
            // TODO we don't clip cairo stuff not in the damaged region. This might be a perf win?
            let mut piet = Piet::new(&ctx);
            // Actually paint the new frame
            let region = self.damaged_region.borrow();
            // The handler must not be already borrowed. This may mean deferring this call.
            self.handler.borrow_mut().paint(&mut piet, &*region);
        }
        // reset damage ready for next frame.
        self.damaged_region.borrow_mut().clear();

        self.buffers.attach();
        self.wl_surface.commit();
    }

    /// Request invalidation of the entire window contents.
    fn invalidate(&self) {
        // This is one of 2 methods the user can use to schedule a repaint, the other is
        // `invalidate_rect`.
        let window_rect = self.logical_size.get().to_rect();
        self.damaged_region.borrow_mut().add_rect(window_rect);
        self.schedule_paint();
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    fn invalidate_rect(&self, rect: Rect) {
        // Quick check to see if we can skip the rect entirely (if it is outside the visible
        // screen).
        if rect.intersect(self.logical_size.get().to_rect()).is_empty() {
            return;
        }
        /* this would be useful for debugging over-keen invalidation by clients.
        println!(
            "{:?} {:?}",
            rect,
            self.with_data(|data| data.logical_size.to_rect())
        );
        */
        self.damaged_region.borrow_mut().add_rect(rect);
        self.schedule_paint()
    }

    /// If there are any pending pointer events, get the next one.
    pub(crate) fn pop_pointer_event(&self) -> Option<MouseEvtKind> {
        self.pointer.borrow_mut().as_mut()?.next()
    }

    /// Initialize the pointer struct, and return the surface used for the cursor.
    pub(crate) fn init_pointer(&self, pointer: WlPointer, serial: u32) {
        if let Some(app_data) = self.app_data.upgrade() {
            let cursor = app_data.wl_compositor.create_surface();
            // TODO for now we've hard-coded 2x scale. This should be dynamic.
            //cursor.set_buffer_scale(2);
            // ignore all events
            cursor.quick_assign(|_, _, _| ());
            *self.pointer.borrow_mut() = Some(Pointer::new(cursor, pointer, serial));
        }
    }

    fn set_system_cursor(&mut self, cursor: Cursor) {
        if let Some(app_data) = self.app_data.upgrade() {
            let cursor = match cursor {
                // TODO check these are all correct
                Cursor::Arrow => "left_ptr",
                Cursor::IBeam => "xterm",
                Cursor::Crosshair => "cross",
                Cursor::OpenHand => "openhand",
                Cursor::NotAllowed => "X_cursor",
                Cursor::ResizeLeftRight => "row-resize",
                Cursor::ResizeUpDown => "col-resize",
                // TODO custom cursors
                _ => "left_ptr",
            };
            let mut theme = app_data.cursor_theme.borrow_mut();
            let cursor = theme.get_cursor(cursor).unwrap();
            // Just use the first image, people using animated cursors have already made bad life
            // choices and shouldn't expect it to work.
            let buf = &cursor[cursor.frame_and_duration(0).frame_index];
            self.set_cursor(buf);
        }
    }
}

/// Builder abstraction for creating new windows
pub(crate) struct WindowBuilder {
    app_data: WeakRc<ApplicationData>,
    handler: Option<Box<dyn WinHandler>>,
    title: String,
    menu: Option<Menu>,
    position: Option<Point>,
    level: Option<WindowLevel>,
    state: Option<window::WindowState>,
    // pre-scaled
    size: Size,
    min_size: Option<Size>,
    resizable: bool,
    show_titlebar: bool,
}

#[derive(Clone)]
pub struct IdleHandle;

#[derive(Clone, PartialEq)]
pub struct CustomCursor;

impl WindowBuilder {
    pub fn new(app: Application) -> WindowBuilder {
        WindowBuilder {
            app_data: Rc::downgrade(&app.data),
            handler: None,
            title: String::new(),
            menu: None,
            size: Size::new(500.0, 400.0),
            position: None,
            level: None,
            state: None,
            min_size: None,
            resizable: true,
            show_titlebar: true,
        }
    }

    pub fn set_handler(&mut self, handler: Box<dyn WinHandler>) {
        self.handler = Some(handler);
    }

    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn set_min_size(&mut self, size: Size) {
        self.min_size = Some(size);
    }

    pub fn resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
    }

    pub fn show_titlebar(&mut self, show_titlebar: bool) {
        self.show_titlebar = show_titlebar;
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        todo!()
    }

    pub fn set_position(&mut self, position: Point) {
        self.position = Some(position);
    }

    pub fn set_level(&mut self, level: WindowLevel) {
        self.level = Some(level);
    }

    pub fn set_window_state(&mut self, state: window::WindowState) {
        self.state = Some(state);
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn set_menu(&mut self, menu: Menu) {
        self.menu = Some(menu);
    }

    pub fn build(self) -> Result<WindowHandle, ShellError> {
        if matches!(self.menu, Some(_)) {
            //panic!("menu unsupported");
        }
        let app_data = match self.app_data.upgrade() {
            Some(app_data) => app_data,
            None => return Err(ShellError::ApplicationDropped),
        };
        let handler = self.handler.expect("must set a window handler");

        let wl_surface = app_data.wl_compositor.create_surface();

        let xdg_surface = app_data.xdg_base.get_xdg_surface(&wl_surface);
        let xdg_toplevel = xdg_surface.get_toplevel();
        let zxdg_toplevel_decoration_v1 = app_data
            .zxdg_decoration_manager_v1
            .get_toplevel_decoration(&xdg_toplevel);
        zxdg_toplevel_decoration_v1.set_mode(DecorationMode::ServerSide);
        xdg_toplevel.set_title(self.title);
        if let Some(size) = self.min_size {
            // for sanity
            assert!(size.width >= 0. && size.height >= 0.);
            xdg_toplevel.set_min_size(size.width as i32, size.height as i32);
        }

        let mut data = Rc::new(WindowData {
            app_data: Rc::downgrade(&app_data),
            wl_surface,
            xdg_surface,
            xdg_toplevel,
            zxdg_toplevel_decoration_v1,
            outputs: RefCell::new(HashSet::new()),
            buffers: Buffers::new(app_data.wl_shm.clone(), self.size.into()),
            logical_size: Cell::new(self.size),
            scale: Cell::new(1),
            keyboard_focus: Cell::new(false),
            pointer: RefCell::new(None),
            anim_frame_requested: Cell::new(false),
            paint_scheduled: Cell::new(false),
            handler: RefCell::new(handler),
            damaged_region: RefCell::new(Region::EMPTY),
        });

        let weak_data = Rc::downgrade(&data);
        // Hook up the child -> parent weak pointer.
        unsafe {
            // Safety: safe because no other references to the data are dereferenced for the life
            // of the reference (the only refs are the Rc and the weak Rc we just created).
            let mut buffers: &mut Buffers<{ NUM_FRAMES as usize }> =
                &mut *(Rc::as_ptr(&data.buffers) as *mut _);
            buffers.set_window_data(weak_data);
        }

        // Insert a reference to us in the application. This is the main strong reference to the
        // app.
        if let Some(old_data) = app_data
            .surfaces
            .borrow_mut()
            .insert(data.id(), data.clone())
        {
            panic!("wayland should use unique object IDs");
        }

        // event handlers
        data.xdg_toplevel.quick_assign(with_cloned!(
            data;
            move |xdg_toplevel, event, _| match event {
                XdgTopLevelEvent::Configure {
                    width,
                    height,
                    states,
                } => {
                    // Only change the size if the passed width/height is non-zero - otherwise we
                    // choose.
                    if width != 0 && height != 0 {
                        // The size here is the logical size
                        let scale = data.scale.get();
                        let raw_logical_size = RawSize { width, height };
                        let logical_size = Size::from(raw_logical_size);
                        if data.logical_size.get() != logical_size {
                            data.logical_size.set(logical_size);
                            data.buffers.set_size(raw_logical_size.scale(scale));
                            // (re-entrancy) Report change to client
                            data.handler.borrow_mut().size(logical_size);
                        }
                        // Check if the client requested a repaint.
                        data.check_for_scheduled_paint();
                    }
                }
                XdgTopLevelEvent::Close => {
                    data.handler.borrow_mut().request_close();
                }
                _ => (),
            }
        ));

        data.zxdg_toplevel_decoration_v1.quick_assign(with_cloned!(
            data;
            move |zxdg_toplevel_decoration_v1, event, _| match event {
                ZxdgToplevelDecorationV1Event::Configure { mode } => {
                    // do nothing for now
                    log::debug!("{:?}", mode);
                }
                _ => (),
            }
        ));

        data.xdg_surface.quick_assign(
            with_cloned!(data; move |xdg_surface, event, _| match event {
                XdgSurfaceEvent::Configure { serial } => {
                    xdg_surface.ack_configure(serial);
                    data.request_paint(); // will also rebuild buffers if needed.
                }
                _ => (),
            }),
        );

        data.wl_surface
            .quick_assign(with_cloned!(data; move |_, event, _| {
                match event {
                    wl_surface::Event::Enter { output } => {
                        data
                            .outputs
                            .borrow_mut()
                            .insert(wl::Proxy::from(output).id());
                    }
                    wl_surface::Event::Leave { output } => {
                        data
                            .outputs
                            .borrow_mut()
                            .remove(&wl::Proxy::from(output).id());
                    }
                    _ => (),
                }
                let new_scale = data.recompute_scale();
                if data.set_scale(new_scale).is_changed() {
                    data.wl_surface.set_buffer_scale(new_scale);
                    // We also need to change the physical size to match the new scale
                    data.set_physical_size(RawSize::from(data.logical_size.get()).scale(new_scale));
                    // always repaint, because the scale changed.
                    data.schedule_paint();
                }
            }));

        // Notify wayland that we've finished setting up.
        data.wl_surface.commit();

        let handle = WindowHandle {
            data: Rc::downgrade(&data),
        };
        data.handler.borrow_mut().connect(&handle.clone().into());

        Ok(handle)
    }
}

impl IdleHandle {
    /// Add an idle handler, which is called (once) when the message loop
    /// is empty. The idle handler will be run from the main UI thread, and
    /// won't be scheduled if the associated view has been dropped.
    ///
    /// Note: the name "idle" suggests that it will be scheduled with a lower
    /// priority than other UI events, but that's not necessarily the case.
    pub fn add_idle_callback<F>(&self, callback: F)
    where
        F: FnOnce(&dyn Any) + Send + 'static,
    {
        todo!()
    }

    pub fn add_idle_token(&self, token: IdleToken) {
        todo!()
    }
}
