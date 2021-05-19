use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::kurbo::Size;
use crate::piet::Piet;
use druid_shell::{
    text::InputHandler, Application, FileDialogToken, FileInfo, IdleToken, KeyEvent, MouseEvent,
    Region, Scale, TextFieldToken, TimerToken, WinHandler, WindowHandle,
};

use super::Window;

struct ShellHandler {
    inner: WindowConnection,
}

enum WindowConnection {
    Waiting,
    Connected(Rc<RefCell<Window>>),
    Closed,
}

impl ShellHandler {
    fn with_window<R>(&self, f: impl FnOnce(&Window) -> R) -> Option<R> {
        match &self.inner {
            WindowConnection::Connected(w) => Some(f(&*w.borrow())),
            _ => {
                eprintln!("missing window");
                None
            }
        }
    }

    fn with_window_mut<R>(&mut self, f: impl FnOnce(&mut Window) -> R) -> Option<R> {
        match &mut self.inner {
            WindowConnection::Connected(w) => Some(f(&mut *w.borrow_mut())),
            _ => {
                eprintln!("missing window");
                None
            }
        }
    }
}

impl WinHandler for ShellHandler {
    fn connect(&mut self, handle: &WindowHandle) {
        self.inner = match self.inner {
            WindowConnection::Waiting => {
                WindowConnection::Connected(Rc::new(RefCell::new(Window::new(handle.clone()))))
            }
            WindowConnection::Connected(_) => panic!("window already connected"),
            WindowConnection::Closed => panic!("window has been closed"),
        };
        self.with_window_mut(|w| w.window_connected());
    }

    fn prepare_paint(&mut self) {
        self.with_window_mut(Window::prepare_paint);
    }

    fn paint(&mut self, piet: &mut Piet, region: &Region) {
        self.with_window_mut(|w| w.paint(piet, region));
        //self.app_state.paint_window(self.window_id, piet, region);
    }

    fn size(&mut self, size: Size) {
        self.with_window_mut(|w| w.size_changed(size));
        //let event = Event::WindowSize(size);
        //self.app_state.do_window_event(event, self.window_id);
    }

    fn scale(&mut self, _scale: Scale) {
        // TODO: Do something with the scale
    }

    fn command(&mut self, _id: u32) {
        //self.app_state.handle_system_cmd(id, Some(self.window_id));
    }

    fn save_as(&mut self, _token: FileDialogToken, _file_info: Option<FileInfo>) {
        //self.app_state.handle_dialog_response(token, file_info);
    }

    fn open_file(&mut self, _token: FileDialogToken, _file_info: Option<FileInfo>) {
        //self.app_state.handle_dialog_response(token, file_info);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        // TODO: double-click detection (or is this done in druid-shell?)
        //let event = Event::MouseDown(event.clone().into());
        //self.app_state.do_window_event(event, self.window_id);
        self.with_window_mut(|w| w.mouse_down(event));
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        self.with_window_mut(|w| w.mouse_up(event));
        //let event = Event::MouseUp(event.clone().into());
        //self.app_state.do_window_event(event, self.window_id);
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        self.with_window_mut(|w| w.mouse_move(event));
        //let event = Event::MouseMove(event.clone().into());
        //self.app_state.do_window_event(event, self.window_id);
    }

    fn mouse_leave(&mut self) {
        //self.app_state
        //.do_window_event(Event::Internal(InternalEvent::MouseLeave), self.window_id);
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        self.with_window_mut(|w| w.key_down(event)).unwrap_or(false)
        //self.app_state
        //.do_window_event(Event::KeyDown(event), self.window_id)
        //.is_handled()
    }

    fn key_up(&mut self, event: KeyEvent) {
        self.with_window_mut(|w| w.key_up(event));
        //self.app_state
        //.do_window_event(Event::KeyUp(event), self.window_id);
    }

    fn wheel(&mut self, event: &MouseEvent) {
        self.with_window_mut(|w| w.scroll(event));
        //self.app_state
        //.do_window_event(Event::Wheel(event.clone().into()), self.window_id);
    }

    fn zoom(&mut self, _delta: f64) {
        //self.with_window_mut(|w| w.mouse_wheel(event))
        //let event = Event::Zoom(delta);
        //self.app_state.do_window_event(event, self.window_id);
    }

    fn got_focus(&mut self) {
        //self.app_state.window_got_focus(self.window_id);
    }

    fn timer(&mut self, token: TimerToken) {
        self.with_window_mut(|w| w.timer(token));
        //self.app_state
        //.do_window_event(Event::Timer(token), self.window_id);
    }

    fn idle(&mut self, token: IdleToken) {
        self.with_window_mut(|w| w.idle(token));
        //self.app_state.idle(token);
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    //fn acquire_input_lock(
    //&mut self,
    //token: TextFieldToken,
    //mutable: bool,
    //) -> Box<dyn InputHandler> {
    ////self.app_state
    ////.inner
    ////.borrow_mut()
    ////.get_ime_lock(self.window_id, token, mutable)
    //}

    fn release_input_lock(&mut self, _token: TextFieldToken) {
        //self.app_state.release_ime_lock(self.window_id, token);
    }

    fn request_close(&mut self) {

        //self.app_state
        //.handle_cmd(sys_cmd::CLOSE_WINDOW.to(self.window_id));
        //self.app_state.process_commands();
        //self.app_state.inner.borrow_mut().do_update();
    }

    fn destroy(&mut self) {
        self.inner = WindowConnection::Closed;
        //self.app_state.remove_window(self.window_id);
    }
}
