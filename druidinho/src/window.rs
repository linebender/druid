use crate::kurbo::Size;
use crate::piet::Piet;
use druid_shell::{IdleToken, KeyEvent, MouseEvent, Region, TimerToken, WindowHandle};

pub struct Window {
    handle: WindowHandle,
}

impl Window {
    pub fn new(handle: WindowHandle) -> Self {
        Window { handle }
    }

    pub fn window_connected(&mut self) {}

    pub fn prepare_paint(&mut self) {}

    pub fn paint(&mut self, piet: &mut Piet, region: &Region) {}

    pub fn size_changed(&mut self, new_size: Size) {}

    pub fn mouse_down(&mut self, event: &MouseEvent) {}

    pub fn mouse_up(&mut self, event: &MouseEvent) {}

    pub fn mouse_move(&mut self, event: &MouseEvent) {}

    pub fn scroll(&mut self, event: &MouseEvent) {}

    pub fn key_down(&mut self, event: KeyEvent) -> bool {
        false
    }

    pub fn key_up(&mut self, event: KeyEvent) {}

    pub fn timer(&mut self, token: TimerToken) {}

    pub fn idle(&mut self, token: IdleToken) {}
}
