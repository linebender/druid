use kurbo::{Rect, Size};
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
    cell::{Cell, RefCell},
    convert::{TryFrom, TryInto},
    fmt,
    ops::{Deref, DerefMut},
    os::{raw::c_void, unix::prelude::RawFd},
    ptr::{self, NonNull},
    rc::{Rc, Weak as WeakRc},
    slice,
};
use wayland_client::{
    self as wl,
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};

use super::{window::WindowData, PIXEL_WIDTH};

/// A collection of buffers that can change size.
///
/// This object knows nothing about scaling or events. It just provides buffers to draw into.
pub struct Buffers<const N: usize> {
    /// The actual buffer objects.
    buffers: Cell<Option<[Buffer; N]>>,
    /// Which buffer is the next to present. Iterates through to `N-1` then wraps. Draw to this
    /// buffer
    pending: Cell<usize>,
    /// The physical size of the buffers.
    ///
    /// This will be different from the buffers' actual size if `recrate_buffers` is true.
    // NOTE: This really should support fractional scaling, use unstable protocol.
    size: Cell<RawSize>,
    /// Do we need to rebuild the framebuffers (size changed).
    recreate_buffers: Cell<bool>,
    /// A paint call could not be completed because no buffers were available. Once a buffer
    /// becomes free, a frame should be painted.
    deferred_paint: Cell<bool>,
    /// This flag allows us to check that we only hand out a mutable ref to the buffer data once.
    /// Otherwise providing mutable access to the data would be unsafe.
    pending_buffer_borrowed: Cell<bool>,
    /// A handle to the `WindowData`, so we can run the paint method.
    ///
    /// Weak handle because logically we are owned by the `WindowData`. If ownership went in both
    /// directions we would leak memory.
    window: WeakRc<WindowData>,
    /// Shared memory to allocate buffers in
    shm: RefCell<Shm>,
}

impl<const N: usize> Buffers<N> {
    /// Create a new `Buffers` object.
    ///
    /// The initial size should have non-zero area.
    pub fn new(wl_shm: wl::Main<WlShm>, size: RawSize) -> Rc<Self> {
        assert!(N >= 2, "must be at least 2 buffers");
        assert!(!size.is_empty(), "window size must not be empty");
        Rc::new(Self {
            buffers: Cell::new(None),
            pending: Cell::new(0),
            size: Cell::new(size),
            recreate_buffers: Cell::new(true),
            deferred_paint: Cell::new(false),
            pending_buffer_borrowed: Cell::new(false),
            window: WeakRc::new(),
            shm: RefCell::new(Shm::new(wl_shm).expect("error allocating shared memory")),
        })
    }

    pub fn set_window_data(&mut self, data: WeakRc<WindowData>) {
        self.window = data;
    }

    /// Get the physical size of the buffer.
    pub fn size(&self) -> RawSize {
        self.size.get()
    }

    /// Request that the size of the buffer is changed.
    pub fn set_size(&self, new_size: RawSize) {
        assert!(!new_size.is_empty(), "window size must not be empty");
        if self.size.get() != new_size {
            self.size.set(new_size);
            self.recreate_buffers.set(true);
        }
    }

    /// Request painting the next frame.
    ///
    /// This calls into user code. To avoid re-entrancy, ensure that we are not already in user
    /// code (defer this call if necessary).
    ///
    /// We will call into `WindowData` to paint the frame, and present it. If no buffers are
    /// available we will set a flag, so that when one becomes available we immediately paint and
    /// present. This includes if we need to resize.
    pub fn request_paint(self: &Rc<Self>) {
        //println!("request paint");
        if self.pending_buffer_borrowed.get() {
            panic!("called request_paint during painting");
        }
        // Ok so complicated here:
        //  - If we need to recreate the buffers, we must wait until *all* buffers are
        //    released, so we can re-use the memory.
        //  - If we are just waiting for the next frame, we can check if the *pending*
        //    buffer is free.
        if self.recreate_buffers.get() {
            // If all buffers are free, destroy and recreate them
            if self.all_buffers_released() {
                //log::debug!("all buffers released, recreating");
                self.deferred_paint.set(false);
                self.recreate_buffers_unchecked();
                self.paint_unchecked();
            } else {
                self.deferred_paint.set(true);
            }
        } else {
            // If the next buffer is free, draw & present. If buffers have not been initialized it
            // is a bug in this code.
            if self.pending_buffer_released() {
                //log::debug!("next frame has been released: draw and present");
                self.deferred_paint.set(false);
                self.paint_unchecked();
            } else {
                self.deferred_paint.set(true);
            }
        }
    }

    /// Paint the next frame, without checking if the buffer is free.
    ///
    /// TODO call `handler.prepare_paint` before setting up cairo & invalidating regions.
    fn paint_unchecked(self: &Rc<Self>) {
        //println!("paint unchecked");
        debug_assert!(!self.size.get().is_empty());
        let mut buf_data = self.pending_buffer_data().unwrap();
        debug_assert!(
            self.pending_buffer_released(),
            "buffer in use/not initialized"
        );
        if let Some(data) = self.window.upgrade() {
            data.paint(self.size.get(), &mut *buf_data, self.recreate_buffers.get());
        }
        self.recreate_buffers.set(false);
    }

    /// Destroy the current buffers, resize the shared memory pool if necessary, and create new
    /// buffers. Does not check if all buffers are free.
    fn recreate_buffers_unchecked(&self) {
        //println!("recreate buffers unchecked");
        debug_assert!(
            self.all_buffers_released(),
            "recreate buffers: some buffer still in use"
        );
        debug_assert!(!self.pending_buffer_borrowed.get());
        self.with_buffers(|buffers| {
            if let Some(buffers) = buffers.as_ref() {
                buffers[0].destroy();
                buffers[1].destroy();
            }
        });
        let new_buffer_size = self.size.get().buffer_size(N.try_into().unwrap());
        // This is probably OOM if it fails, but we unwrap to report the underlying error.
        self.shm.borrow_mut().extend(new_buffer_size).unwrap();

        let pool = self.shm.borrow_mut().create_pool();
        self.buffers.set({
            let mut buffers = vec![];

            let size = self.size.get();
            for i in 0..N {
                buffers.push(Buffer::create(
                    &pool,
                    self.window.clone(),
                    i,
                    size.width,
                    size.height,
                ));
            }
            Some(buffers.try_into().unwrap())
        });
        pool.destroy();
        // Don't unset `recreate_buffers` here. We immediately call paint_unchecked, and need to
        // know if buffers were recreated (to invalidate the whole window).
    }

    fn with_buffers<T>(&self, f: impl FnOnce(&Option<[Buffer; N]>) -> T) -> T {
        let buffers = self.buffers.replace(None);
        let out = f(&buffers);
        self.buffers.set(buffers);
        out
    }

    /// Get a ref to the next buffer to draw to.
    fn with_pending_buffer<T>(&self, f: impl FnOnce(Option<&Buffer>) -> T) -> T {
        self.with_buffers(|buffers| f(buffers.as_ref().map(|buffers| &buffers[self.pending.get()])))
    }

    /// For checking whether all buffers are free.
    fn all_buffers_released(&self) -> bool {
        self.with_buffers(|buffers| {
            buffers
                .as_ref()
                .map(|buffers| buffers.iter().all(|buf| !buf.in_use.get()))
                .unwrap_or(true)
        })
    }

    /// For checking whether the next buffer is free.
    fn pending_buffer_released(&self) -> bool {
        self.with_pending_buffer(|buf| buf.map(|buf| !buf.in_use.get()).unwrap_or(false))
    }

    /// Get the raw buffer data of the next buffer to draw to.
    ///
    /// Will return `None` if buffer already borrowed.
    fn pending_buffer_data<'a>(self: &'a Rc<Self>) -> Option<impl DerefMut<Target = [u8]> + 'a> {
        if self.pending_buffer_borrowed.get() {
            None
        } else {
            self.pending_buffer_borrowed.set(true);
            let frame_len = self.frame_len();
            // Safety: we make sure the data is only loaned out once.
            unsafe {
                Some(BufferData {
                    buffers: Rc::downgrade(self),
                    mmap: self
                        .shm
                        .borrow()
                        .mmap(frame_len * self.pending.get(), frame_len),
                })
            }
        }
    }

    /// Signal to wayland that the pending buffer is ready to be presented, and switch the next
    /// buffer to be the pending one.
    pub(crate) fn attach(&self) {
        if let Some(data) = self.window.upgrade() {
            self.with_pending_buffer(|buf| buf.unwrap().attach(&data.wl_surface));
            self.pending.set((self.pending.get() + 1) % N);
        }
    }

    fn frame_len(&self) -> usize {
        let size = self.size.get();
        (PIXEL_WIDTH * size.width * size.height)
            .try_into()
            .expect("integer overflow")
    }
}

/// A wrapper round `WlBuffer` that tracks whether the buffer is released.
///
/// No allocations on `clone`.
#[derive(Debug, Clone)]
pub struct Buffer {
    inner: wl::Main<WlBuffer>,
    in_use: Rc<Cell<bool>>,
}

impl Buffer {
    /// Create a new buffer using the given backing storage. It is the responsibility of the caller
    /// to ensure buffers don't overlap, and the backing storage has enough space.
    // Window handle is needed for the callback.
    pub fn create(
        pool: &wl::Main<WlShmPool>,
        window: WeakRc<WindowData>,
        idx: usize,
        width: i32,
        height: i32,
    ) -> Self {
        // TODO overflow
        let offset = i32::try_from(idx).unwrap() * width * height * PIXEL_WIDTH;
        let stride = width * PIXEL_WIDTH;
        let inner = pool.create_buffer(offset, width, height, stride, wl_shm::Format::Argb8888);
        let in_use = Rc::new(Cell::new(false));

        inner.quick_assign(with_cloned!(in_use; move |_, event, _| match event {
            wl_buffer::Event::Release => {
                in_use.set(false);
                // TODO look at this. We should only paint if it has been requested.
                if let Some(data) = window.upgrade() {
                    data.buffers.request_paint();
                }
            }
            _ => (), // panic!("release is the only event"),
        }));

        Buffer { inner, in_use }
    }

    pub fn attach(&self, wl_surface: &wl::Main<WlSurface>) {
        if self.in_use.get() {
            panic!("attaching an already in-use surface");
        }
        self.in_use.set(true);
        wl_surface.attach(Some(&self.inner), 0, 0);
    }

    pub fn destroy(&self) {
        if self.in_use.get() {
            panic!("Destroying a buffer while it is in use");
        }
        self.inner.destroy();
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.destroy();
    }
}

pub struct BufferData<const N: usize> {
    buffers: WeakRc<Buffers<N>>,
    mmap: Mmap,
}

impl<const N: usize> Deref for BufferData<N> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.mmap.deref()
    }
}

impl<const N: usize> DerefMut for BufferData<N> {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.mmap.deref_mut()
    }
}

impl<const N: usize> Drop for BufferData<N> {
    fn drop(&mut self) {
        if let Some(buffers) = self.buffers.upgrade() {
            buffers.pending_buffer_borrowed.set(false);
        }
    }
}

/// RAII wrapper for shm_open (file descriptors for mmap'd shared memory)
///
/// Designed to work like a vec: to manage extending when necessary.
pub struct Shm {
    inner: RawFd,
    size: usize,
    // a handle on the wayland structure.
    wl_shm: wl::Main<WlShm>,
}

impl Shm {
    /// Create a new shared memory object. Will be empty until resized.
    pub fn new(wl_shm: wl::Main<WlShm>) -> Result<Self, nix::Error> {
        // TODO is this a good way to choose a filename? What should our retry strategy be?
        let name = format!("/druid-wl-{}", rand::random::<i32>());
        // Open the file we will use for shared memory.
        let fd = shm_open(
            name.as_str(),
            OFlag::O_RDWR | OFlag::O_EXCL | OFlag::O_CREAT,
            Mode::S_IRUSR | Mode::S_IWUSR,
        )?;

        // The memory is 0-sized until we resize it with `ftruncate`.
        let shm = Shm {
            inner: fd,
            size: 0,
            wl_shm,
        };
        Ok(shm)
    }

    /// Resizes the shared memory pool.
    ///
    /// This is almost certainly unsafe if the server is using the memory TODO use locking
    /// (provided by wayland I think).
    pub fn resize(&mut self, new_size: i32) -> Result<(), nix::Error> {
        let new_size: usize = new_size.try_into().unwrap();
        if self.size == new_size {
            return Ok(());
        }

        // allocate the space (retry on interrupt)
        loop {
            match ftruncate(self.inner, new_size.try_into().unwrap()) {
                Ok(()) => {
                    self.size = new_size;
                    return Ok(());
                }
                Err(e) if matches!(e.as_errno(), Some(Errno::EINTR)) => {
                    // continue (try again)
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    /// Like `resize`, but doesn't shrink.
    pub fn extend(&mut self, new_size: i32) -> Result<(), nix::Error> {
        if self.size < new_size.try_into().unwrap() {
            self.resize(new_size)
        } else {
            Ok(())
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Create a `WlShmPool` backed by our memory that will be mmap'd by the server.
    pub fn create_pool(&self) -> wl::Main<WlShmPool> {
        self.wl_shm
            .create_pool(self.inner, self.size.try_into().unwrap())
    }

    /// A method to make all the data `1` (white). Useful for debugging.
    ///
    /// Safe only when no frames are in use.
    #[allow(unused)]
    pub fn fill_white(&mut self) {
        unsafe {
            let mut buf = self.mmap(0, self.size);
            for byte in buf.as_mut() {
                *byte = 0xff;
            }
        }
    }

    /// Get access to the shared memory for the given frame.
    ///
    /// # Safety
    ///
    /// It's not checked if any other process has access to the memory. Data races may occur if
    /// they do.
    pub unsafe fn mmap(&self, offset: usize, len: usize) -> Mmap {
        Mmap::from_raw(self.inner, self.size, offset, len).unwrap()
    }

    /// Closing with error checking
    pub fn close(self) -> Result<(), nix::Error> {
        close(self.inner)
    }
}

impl Drop for Shm {
    fn drop(&mut self) {
        // cannot handle errors in drop.
        let _ = close(self.inner);
    }
}

pub struct Mmap {
    ptr: NonNull<c_void>,
    size: usize,
    offset: usize,
    len: usize,
}

impl Mmap {
    /// `fd` and `size` are the whole memory you want to map. `offset` and `len` are there to
    /// provide extra protection (only giving you access to that part).
    ///
    /// # Safety
    ///
    /// Concurrent use of the memory we map to isn't checked.
    #[inline]
    pub unsafe fn from_raw(
        fd: RawFd,
        size: usize,
        offset: usize,
        len: usize,
    ) -> Result<Self, nix::Error> {
        Self::from_raw_inner(fd, size, offset, len, false)
    }

    #[inline]
    pub unsafe fn from_raw_private(
        fd: RawFd,
        size: usize,
        offset: usize,
        len: usize,
    ) -> Result<Self, nix::Error> {
        Self::from_raw_inner(fd, size, offset, len, true)
    }

    unsafe fn from_raw_inner(
        fd: RawFd,
        size: usize,
        offset: usize,
        len: usize,
        private: bool,
    ) -> Result<Self, nix::Error> {
        assert!(offset + len <= size, "{0} + {1} <= {2}", offset, len, size,);
        let map_flags = if private {
            MapFlags::MAP_PRIVATE
        } else {
            MapFlags::MAP_SHARED
        };
        let ptr = mmap(
            ptr::null_mut(),
            size,
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            map_flags,
            fd,
            0,
        )?;
        Ok(Mmap {
            ptr: NonNull::new(ptr).unwrap(),
            size,
            offset,
            len,
        })
    }
}

impl Deref for Mmap {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe {
            let start = self.ptr.as_ptr().offset(self.offset.try_into().unwrap());
            slice::from_raw_parts(start as *const u8, self.len)
        }
    }
}

impl DerefMut for Mmap {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            let start = self.ptr.as_ptr().offset(self.offset.try_into().unwrap());
            slice::from_raw_parts_mut(start as *mut u8, self.len)
        }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        unsafe {
            if let Err(e) = munmap(self.ptr.as_ptr(), self.size) {
                log::warn!("Error unmapping memory: {}", e);
            }
        }
    }
}
#[derive(Copy, Clone, PartialEq)]
pub struct RawSize {
    pub width: i32,
    pub height: i32,
}

impl RawSize {
    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    /// How many bytes do we need to store a frame of this size (in pixels)
    pub fn frame_size(self) -> i32 {
        // Check for overflow
        assert!(self.width.checked_mul(self.height).unwrap() < i32::MAX / PIXEL_WIDTH);
        self.width * self.height * PIXEL_WIDTH
    }

    /// Helper function to get the total buffer size we will need for all the frames.
    pub fn buffer_size(self, frames: i32) -> i32 {
        // Check for overflow
        assert!(self.width.checked_mul(self.height).unwrap() < i32::MAX / (PIXEL_WIDTH * frames));
        self.width * self.height * PIXEL_WIDTH * frames
    }

    pub fn scale(self, scale: i32) -> Self {
        // NOTE no overflow checking atm.
        RawSize {
            width: self.width * scale,
            height: self.height * scale,
        }
    }

    pub fn to_rect(self) -> RawRect {
        RawRect {
            x0: 0,
            y0: 0,
            x1: self.width,
            y1: self.height,
        }
    }

    pub fn area(self) -> i32 {
        self.width * self.height
    }

    pub fn is_empty(self) -> bool {
        self.area() == 0
    }
}

impl From<Size> for RawSize {
    fn from(s: Size) -> Self {
        let width = s.width as i32;
        let height = s.height as i32;

        // Sanity check
        assert!(width >= 0 && height >= 0);

        RawSize { width, height }
    }
}

impl From<RawSize> for Size {
    fn from(s: RawSize) -> Self {
        Size::new(s.width as f64, s.height as f64)
    }
}

impl fmt::Debug for RawSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}×{}", self.width, self.height)
    }
}

#[derive(Debug)]
pub struct RawRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl RawRect {
    pub fn scale(self, scale: i32) -> Self {
        // NOTE no overflow checking atm.
        RawRect {
            x0: self.x0 * scale,
            y0: self.y0 * scale,
            x1: self.x1 * scale,
            y1: self.y1 * scale,
        }
    }
}

impl From<Rect> for RawRect {
    fn from(r: Rect) -> Self {
        let max = i32::MAX as f64;
        assert!(r.x0.abs() < max && r.y0.abs() < max && r.x1.abs() < max && r.y1.abs() < max);
        RawRect {
            x0: r.x0 as i32,
            y0: r.y0 as i32,
            x1: r.x1 as i32,
            y1: r.y1 as i32,
        }
    }
}

impl From<RawRect> for Rect {
    fn from(r: RawRect) -> Self {
        Rect {
            x0: r.x0 as f64,
            y0: r.y0 as f64,
            x1: r.x1 as f64,
            y1: r.y1 as f64,
        }
    }
}
