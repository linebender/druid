// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wayland_client as wlc;
use wayland_client::protocol::wl_surface;
use wayland_protocols::xdg_shell::client::xdg_popup;
use wayland_protocols::xdg_shell::client::xdg_positioner;
use wayland_protocols::xdg_shell::client::xdg_surface;
use wlc::protocol::wl_region::WlRegion;

use crate::kurbo;
use crate::window;
use crate::{piet::Piet, region::Region, scale::Scale, TextFieldToken};

use super::super::Changed;

use super::super::outputs;
use super::buffers;
use super::error;
use super::idle;
use super::Popup;
use super::{Compositor, CompositorHandle, Decor, Handle, Outputs};

pub enum DeferredTask {
    Paint,
    AnimationClear,
}

#[derive(Clone)]
pub struct Surface {
    pub(super) inner: std::sync::Arc<Data>,
}

impl From<std::sync::Arc<Data>> for Surface {
    fn from(d: std::sync::Arc<Data>) -> Self {
        Self { inner: d }
    }
}

impl Surface {
    pub fn new(
        c: impl Into<CompositorHandle>,
        handler: Box<dyn window::WinHandler>,
        initial_size: kurbo::Size,
    ) -> Self {
        let compositor = CompositorHandle::new(c);
        let wl_surface = match compositor.create_surface() {
            None => panic!("unable to create surface"),
            Some(v) => v,
        };

        let current = std::sync::Arc::new(Data {
            compositor: compositor.clone(),
            wl_surface: RefCell::new(wl_surface),
            outputs: RefCell::new(std::collections::HashSet::new()),
            buffers: buffers::Buffers::new(compositor.shared_mem(), initial_size.into()),
            logical_size: Cell::new(initial_size),
            scale: Cell::new(1),
            anim_frame_requested: Cell::new(false),
            handler: RefCell::new(handler),
            idle_queue: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
            active_text_input: Cell::new(None),
            damaged_region: RefCell::new(Region::EMPTY),
            deferred_tasks: RefCell::new(std::collections::VecDeque::new()),
        });

        // register to receive wl_surface events.
        Surface::initsurface(&current);

        Self { inner: current }
    }

    pub(super) fn output(&self) -> Option<outputs::Meta> {
        self.inner.output()
    }

    pub(super) fn request_paint(&self) {
        self.inner.buffers.request_paint(&self.inner);
    }

    pub(super) fn update_dimensions(&self, dim: impl Into<kurbo::Size>) -> kurbo::Size {
        self.inner.update_dimensions(dim)
    }

    pub(super) fn resize(&self, dim: kurbo::Size) -> kurbo::Size {
        self.inner.resize(dim)
    }

    pub(super) fn commit(&self) {
        self.inner.wl_surface.borrow().commit()
    }

    pub(super) fn replace(current: &std::sync::Arc<Data>) -> Surface {
        current
            .wl_surface
            .replace(match current.compositor.create_surface() {
                None => panic!("unable to create surface"),
                Some(v) => v,
            });
        Surface::initsurface(current);
        Self {
            inner: current.clone(),
        }
    }

    fn initsurface(current: &std::sync::Arc<Data>) {
        current.wl_surface.borrow().quick_assign({
            let current = current.clone();
            move |a, event, b| {
                tracing::debug!("wl_surface event {:?} {:?} {:?}", a, event, b);
                Surface::consume_surface_event(&current, &a, &event, &b);
            }
        });
    }

    pub(super) fn consume_surface_event(
        current: &std::sync::Arc<Data>,
        surface: &wlc::Main<wlc::protocol::wl_surface::WlSurface>,
        event: &wlc::protocol::wl_surface::Event,
        data: &wlc::DispatchData,
    ) {
        tracing::debug!("wl_surface event {:?} {:?} {:?}", surface, event, data);
        match event {
            wl_surface::Event::Enter { output } => {
                let proxy = wlc::Proxy::from(output.clone());
                current.outputs.borrow_mut().insert(proxy.id());
            }
            wl_surface::Event::Leave { output } => {
                let proxy = wlc::Proxy::from(output.clone());
                current.outputs.borrow_mut().remove(&proxy.id());
            }
            _ => tracing::warn!("unhandled wayland surface event {:?}", event),
        }

        if current.wl_surface.borrow().as_ref().version() >= wl_surface::REQ_SET_BUFFER_SCALE_SINCE
        {
            let new_scale = current.recompute_scale();
            if current.set_scale(new_scale).is_changed() {
                current.wl_surface.borrow().set_buffer_scale(new_scale);
                // We also need to change the physical size to match the new scale
                current
                    .buffers
                    .set_size(buffers::RawSize::from(current.logical_size.get()).scale(new_scale));
                // always repaint, because the scale changed.
                current.schedule_deferred_task(DeferredTask::Paint);
            }
        }
    }
}

impl Outputs for Surface {
    fn removed(&self, o: &outputs::Meta) {
        self.inner.outputs.borrow_mut().remove(&o.id());
    }

    fn inserted(&self, _: &outputs::Meta) {
        // nothing to do here.
    }
}

impl Handle for Surface {
    fn get_size(&self) -> kurbo::Size {
        self.inner.get_size()
    }

    fn set_size(&self, dim: kurbo::Size) {
        self.inner.resize(dim);
    }

    fn request_anim_frame(&self) {
        self.inner.request_anim_frame()
    }

    fn remove_text_field(&self, token: TextFieldToken) {
        self.inner.remove_text_field(token)
    }

    fn set_focused_text_field(&self, active_field: Option<TextFieldToken>) {
        self.inner.set_focused_text_field(active_field)
    }

    fn set_input_region(&self, region: Option<Region>) {
        self.inner.set_interactable_region(region);
    }

    fn get_idle_handle(&self) -> idle::Handle {
        self.inner.get_idle_handle()
    }

    fn get_scale(&self) -> Scale {
        self.inner.get_scale()
    }

    fn invalidate(&self) {
        self.inner.invalidate()
    }

    fn invalidate_rect(&self, rect: kurbo::Rect) {
        self.inner.invalidate_rect(rect)
    }

    fn run_idle(&self) {
        self.inner.run_idle();
    }

    fn release(&self) {
        self.inner.release()
    }

    fn data(&self) -> Option<std::sync::Arc<Data>> {
        Some(Into::into(self))
    }
}

impl From<Surface> for std::sync::Arc<Data> {
    fn from(s: Surface) -> std::sync::Arc<Data> {
        s.inner
    }
}

impl From<&Surface> for std::sync::Arc<Data> {
    fn from(s: &Surface) -> std::sync::Arc<Data> {
        s.inner.clone()
    }
}

pub struct Data {
    pub(super) compositor: CompositorHandle,
    pub(super) wl_surface: RefCell<wlc::Main<wl_surface::WlSurface>>,

    /// The outputs that our surface is present on (we should get the first enter event early).
    pub(super) outputs: RefCell<std::collections::HashSet<u32>>,

    /// Buffers in our shared memory.
    // Buffers sometimes need to move references to themselves into closures, so must be behind a
    // reference counter.
    pub(super) buffers: Rc<buffers::Buffers<{ buffers::NUM_FRAMES as usize }>>,
    /// The logical size of the next frame.
    pub(crate) logical_size: Cell<kurbo::Size>,
    /// The scale we are rendering to (defaults to 1)
    pub(crate) scale: Cell<i32>,

    /// Contains the callbacks from user code.
    pub(crate) handler: RefCell<Box<dyn window::WinHandler>>,
    pub(crate) active_text_input: Cell<Option<TextFieldToken>>,

    /// Whether we have requested an animation frame. This stops us requesting more than 1.
    anim_frame_requested: Cell<bool>,
    /// Rects of the image that are damaged and need repainting in the logical coordinate space.
    ///
    /// This lives outside `data` because they can be borrowed concurrently without re-entrancy.
    damaged_region: RefCell<Region>,
    /// Tasks that were requested in user code.
    ///
    /// These call back into user code, and so should only be run after all user code has returned,
    /// to avoid possible re-entrancy.
    deferred_tasks: RefCell<std::collections::VecDeque<DeferredTask>>,

    idle_queue: std::sync::Arc<std::sync::Mutex<Vec<idle::Kind>>>,
}

impl Data {
    pub(crate) fn output(&self) -> Option<outputs::Meta> {
        match self.outputs.borrow().iter().find(|_| true) {
            None => None,
            Some(id) => self.compositor.output(*id),
        }
    }

    #[track_caller]
    pub(crate) fn with_handler<T, F: FnOnce(&mut dyn window::WinHandler) -> T>(
        &self,
        f: F,
    ) -> Option<T> {
        let ret = self.with_handler_and_dont_check_the_other_borrows(f);
        self.run_deferred_tasks();
        ret
    }

    #[track_caller]
    fn with_handler_and_dont_check_the_other_borrows<
        T,
        F: FnOnce(&mut dyn window::WinHandler) -> T,
    >(
        &self,
        f: F,
    ) -> Option<T> {
        match self.handler.try_borrow_mut() {
            Ok(mut h) => Some(f(&mut **h)),
            Err(_) => {
                tracing::error!(
                    "failed to borrow WinHandler at {}",
                    std::panic::Location::caller()
                );
                None
            }
        }
    }

    pub(super) fn update_dimensions(&self, dim: impl Into<kurbo::Size>) -> kurbo::Size {
        let dim = dim.into();
        if self.logical_size.get() != self.resize(dim) {
            match self.handler.try_borrow_mut() {
                Ok(mut handler) => handler.size(dim),
                Err(cause) => tracing::warn!("unhable to borrow handler {:?}", cause),
            };
        }

        dim
    }

    // client initiated resizing.
    pub(super) fn resize(&self, dim: kurbo::Size) -> kurbo::Size {
        // The size here is the logical size
        let scale = self.scale.get();
        let raw_logical_size = buffers::RawSize {
            width: dim.width as i32,
            height: dim.height as i32,
        };
        let previous_logical_size = self.logical_size.replace(dim);
        if previous_logical_size != dim {
            self.buffers.set_size(raw_logical_size.scale(scale));
        }

        dim
    }

    pub(super) fn set_interactable_region(&self, region: Option<Region>) {
        match region {
            Some(region) => {
                let wl_region = self.compositor.create_region();

                let detached_region: WlRegion = wl_region.detach();
                for rect in region.rects() {
                    detached_region.add(
                        rect.x0 as i32,
                        rect.y0 as i32,
                        rect.width().ceil() as i32,
                        rect.height().ceil() as i32,
                    );
                }
                self.wl_surface
                    .borrow()
                    .set_input_region(Some(&detached_region));
                detached_region.destroy();
            }
            None => {
                // This, for some reason, causes a shift in the cursor.
                self.wl_surface.borrow().set_input_region(None);
            }
        }
    }

    /// Assert that the physical size = logical size * scale
    #[allow(unused)]
    fn assert_size(&self) {
        assert_eq!(
            self.buffers.size(),
            buffers::RawSize::from(self.logical_size.get()).scale(self.scale.get()),
            "phy {:?} == logic {:?} * {}",
            self.buffers.size(),
            self.logical_size.get(),
            self.scale.get()
        );
    }

    /// Recompute the scale to use (the maximum of all the scales for the different outputs this
    /// surface is drawn to).
    fn recompute_scale(&self) -> i32 {
        tracing::debug!("recompute initiated");
        self.compositor.recompute_scale(&self.outputs.borrow())
    }

    /// Sets the scale
    ///
    /// Up to the caller to make sure `physical_size`, `logical_size` and `scale` are consistent.
    fn set_scale(&self, new_scale: i32) -> Changed {
        tracing::debug!("set_scale initiated");
        if self.scale.get() != new_scale {
            self.scale.set(new_scale);
            // (re-entrancy) Report change to client
            self.handler
                .borrow_mut()
                .scale(Scale::new(new_scale as f64, new_scale as f64));
            Changed::Changed
        } else {
            Changed::Unchanged
        }
    }

    /// Paint the next frame.
    ///
    /// The buffers object is responsible for calling this function after we called
    /// `request_paint`.
    ///
    /// - `buf` is what we draw the frame into
    /// - `size` is the physical size in pixels we are drawing.
    /// - `force` means draw the whole frame, even if it wasn't all invalidated.
    pub(super) fn paint(&self, physical_size: buffers::RawSize, buf: &mut [u8], force: bool) {
        tracing::trace!(
            "paint initiated {:?} - {:?} {:?}",
            self.get_size(),
            physical_size,
            force
        );

        // We don't care about obscure pre version 4 compositors
        // and just damage the whole surface instead of
        // translating from buffer coordinates to surface coordinates
        let damage_buffer_supported =
            self.wl_surface.borrow().as_ref().version() >= wl_surface::REQ_DAMAGE_BUFFER_SINCE;

        if force || !damage_buffer_supported {
            self.invalidate();
            self.wl_surface.borrow().damage(0, 0, i32::MAX, i32::MAX);
        } else {
            let damaged_region = self.damaged_region.borrow_mut();
            for rect in damaged_region.rects() {
                // Convert it to physical coordinate space.
                let rect = buffers::RawRect::from(*rect).scale(self.scale.get());

                self.wl_surface.borrow().damage_buffer(
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
            // We're going to lie about the lifetime of our buffer here. This is (I think) ok,
            // because the Rust wrapper for cairo is overly pessimistic: the buffer only has to
            // last as long as the `ImageSurface` (which we know this buffer will).
            let buf: &'static mut [u8] = &mut *(buf as *mut _);
            let cairo_surface = match cairo::ImageSurface::create_for_data(
                buf,
                cairo::Format::ARgb32,
                physical_size.width,
                physical_size.height,
                physical_size.width * buffers::PIXEL_WIDTH,
            ) {
                Ok(s) => s,
                Err(cause) => {
                    tracing::error!("unable to create cairo surface: {:?}", cause);
                    return;
                }
            };
            let ctx = match cairo::Context::new(&cairo_surface) {
                Ok(ctx) => ctx,
                Err(cause) => {
                    tracing::error!("unable to create cairo context: {:?}", cause);
                    return;
                }
            };
            // Apply scaling
            let scale = self.scale.get() as f64;
            ctx.scale(scale, scale);

            let mut piet = Piet::new(&ctx);
            // Actually paint the new frame
            let region = self.damaged_region.borrow();

            // The handler must not be already borrowed. This may mean deferring this call.
            self.handler.borrow_mut().paint(&mut piet, &region);
        }

        // reset damage ready for next frame.
        self.damaged_region.borrow_mut().clear();
        self.buffers.attach(self);
        self.wl_surface.borrow().commit();
    }

    /// Request invalidation of the entire window contents.
    fn invalidate(&self) {
        tracing::trace!("invalidate initiated");
        // This is one of 2 methods the user can use to schedule a repaint, the other is
        // `invalidate_rect`.
        let window_rect = self.logical_size.get().to_rect();
        self.damaged_region.borrow_mut().add_rect(window_rect);
        self.schedule_deferred_task(DeferredTask::Paint);
    }

    /// Request invalidation of one rectangle, which is given in display points relative to the
    /// drawing area.
    fn invalidate_rect(&self, rect: kurbo::Rect) {
        tracing::trace!("invalidate_rect initiated {:?}", rect);
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
        self.schedule_deferred_task(DeferredTask::Paint);
    }

    pub fn schedule_deferred_task(&self, task: DeferredTask) {
        tracing::trace!("scedule_deferred_task initiated");
        self.deferred_tasks.borrow_mut().push_back(task);
    }

    pub fn run_deferred_tasks(&self) {
        tracing::trace!("run_deferred_tasks initiated");
        while let Some(task) = self.next_deferred_task() {
            self.run_deferred_task(task);
        }
    }

    fn next_deferred_task(&self) -> Option<DeferredTask> {
        self.deferred_tasks.borrow_mut().pop_front()
    }

    fn run_deferred_task(&self, task: DeferredTask) {
        match task {
            DeferredTask::Paint => {
                self.buffers.request_paint(self);
            }
            DeferredTask::AnimationClear => {
                self.anim_frame_requested.set(false);
            }
        }
    }

    pub(super) fn get_size(&self) -> kurbo::Size {
        // size in pixels, so we must apply scale.
        let logical_size = self.logical_size.get();
        let scale = self.scale.get() as f64;
        kurbo::Size::new(logical_size.width * scale, logical_size.height * scale)
    }

    pub(super) fn request_anim_frame(&self) {
        if self.anim_frame_requested.replace(true) {
            return;
        }

        let idle = self.get_idle_handle();
        idle.add_idle_callback(move |winhandle| {
            winhandle.prepare_paint();
        });
        self.schedule_deferred_task(DeferredTask::AnimationClear);
    }

    pub(super) fn remove_text_field(&self, token: TextFieldToken) {
        if self.active_text_input.get() == Some(token) {
            self.active_text_input.set(None);
        }
    }

    pub(super) fn set_focused_text_field(&self, active_field: Option<TextFieldToken>) {
        self.active_text_input.set(active_field);
    }

    pub(super) fn get_idle_handle(&self) -> idle::Handle {
        idle::Handle {
            queue: self.idle_queue.clone(),
        }
    }

    pub(super) fn get_scale(&self) -> Scale {
        let scale = self.scale.get() as f64;
        Scale::new(scale, scale)
    }

    pub(super) fn run_idle(&self) {
        self.with_handler(|winhandle| {
            idle::run(&self.get_idle_handle(), winhandle);
        });
    }

    pub(super) fn release(&self) {
        self.wl_surface.borrow().destroy();
    }
}

#[derive(Default)]
pub struct Dead;

impl From<Dead> for Box<dyn Decor> {
    fn from(d: Dead) -> Box<dyn Decor> {
        Box::new(d) as Box<dyn Decor>
    }
}

impl From<Dead> for Box<dyn Outputs> {
    fn from(d: Dead) -> Box<dyn Outputs> {
        Box::new(d) as Box<dyn Outputs>
    }
}

impl Decor for Dead {
    fn inner_set_title(&self, title: String) {
        tracing::warn!("set_title not implemented for this surface: {:?}", title);
    }
}

impl Outputs for Dead {
    fn removed(&self, _: &outputs::Meta) {}

    fn inserted(&self, _: &outputs::Meta) {}
}

impl Popup for Dead {
    fn surface<'a>(
        &self,
        _: &'a wlc::Main<xdg_surface::XdgSurface>,
        _: &'a wlc::Main<xdg_positioner::XdgPositioner>,
    ) -> Result<wlc::Main<xdg_popup::XdgPopup>, error::Error> {
        tracing::warn!("popup invoked on a dead surface");
        Err(error::Error::InvalidParent(0))
    }
}

impl Handle for Dead {
    fn get_size(&self) -> kurbo::Size {
        kurbo::Size::ZERO
    }

    fn set_size(&self, dim: kurbo::Size) {
        tracing::warn!("set_size invoked on a dead surface {:?}", dim);
    }

    fn request_anim_frame(&self) {
        tracing::warn!("request_anim_frame invoked on a dead surface")
    }

    fn remove_text_field(&self, _token: TextFieldToken) {
        tracing::warn!("remove_text_field invoked on a dead surface")
    }

    fn set_focused_text_field(&self, _active_field: Option<TextFieldToken>) {
        tracing::warn!("set_focused_text_field invoked on a dead surface")
    }

    fn set_input_region(&self, _region: Option<Region>) {
        tracing::warn!("set_input_region invoked on a dead surface")
    }

    fn get_idle_handle(&self) -> idle::Handle {
        panic!("get_idle_handle invoked on a dead surface")
    }

    fn get_scale(&self) -> Scale {
        Scale::new(1., 1.)
    }

    fn invalidate(&self) {
        tracing::warn!("invalidate invoked on a dead surface")
    }

    fn invalidate_rect(&self, _rect: kurbo::Rect) {
        tracing::warn!("invalidate_rect invoked on a dead surface")
    }

    fn run_idle(&self) {
        tracing::warn!("run_idle invoked on a dead surface")
    }

    fn release(&self) {
        tracing::warn!("release invoked on a dead surface");
    }

    fn data(&self) -> Option<std::sync::Arc<Data>> {
        tracing::warn!("data invoked on a dead surface");
        None
    }
}
