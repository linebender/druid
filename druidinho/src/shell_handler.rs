use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::kurbo::Size;
use crate::piet::Piet;

use druid_shell::{
    FileDialogToken, FileInfo, IdleToken, KeyEvent, MouseEvent, Region, Scale, TextFieldToken,
    TimerToken, WinHandler, WindowHandle,
};

use super::{App, LaunchCtx, Window};

pub struct ShellHandler<T> {
    //app: Application,
    inner: WindowConnection<T>,
}

enum WindowConnection<T> {
    Waiting(Box<dyn App<Action = T>>),
    //Connected(Rc<RefCell<Window<T>>>),
    Connected {
        window: Rc<RefCell<Window<T>>>,
        app: Box<dyn App<Action = T>>,
    },
    Closed,
    // a sentinel state only used during transitions
    Invalid,
}

impl<T> WindowConnection<T> {
    fn transition(&mut self) -> WindowConnection<T> {
        std::mem::replace(self, WindowConnection::Invalid)
    }
}

impl<T> ShellHandler<T> {
    pub fn new(widget: impl App<Action = T> + 'static) -> Self {
        ShellHandler {
            inner: WindowConnection::Waiting(Box::new(widget)),
        }
    }

    fn after_event(&mut self) {
        if let WindowConnection::Connected { app, window } = &mut self.inner {
            let mut needs_update = false;
            if !window.borrow().messages.is_empty() {
                app.update(&window.borrow().messages, &mut needs_update);
                window.borrow_mut().messages.clear();
            }
            if needs_update {
                window.borrow_mut().update();
            }
        }
    }

    //fn with_window<R>(&self, f: impl FnOnce(&Window) -> R) -> Option<R> {
    //match &self.inner {
    //WindowConnection::Connected(w) => Some(f(&*w.borrow())),
    //_ => {
    //eprintln!("missing window");
    //None
    //}
    //}
    //}

    fn with_window_mut<R>(&mut self, f: impl FnOnce(&mut Window<T>) -> R) -> Option<R> {
        match &mut self.inner {
            WindowConnection::Connected { window, .. } => {
                let r = Some(f(&mut *window.borrow_mut()));
                self.after_event();
                r
            }
            _ => {
                eprintln!("missing window");
                None
            }
        }
    }
}

impl<T: 'static> WinHandler for ShellHandler<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.inner = match self.inner.transition() {
            WindowConnection::Waiting(mut app) => {
                let window = Rc::new(RefCell::new(Window::new(
                    handle.clone(),
                    app.launch(&mut LaunchCtx),
                )));
                WindowConnection::Connected { window, app }
            }
            WindowConnection::Connected { .. } => panic!("window already connected"),
            WindowConnection::Closed => panic!("window has been closed"),
            WindowConnection::Invalid => unreachable!(),
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

    fn command(&mut self, _id: u32) {}

    fn save_as(&mut self, _token: FileDialogToken, _file_info: Option<FileInfo>) {}

    fn open_file(&mut self, _token: FileDialogToken, _file_info: Option<FileInfo>) {}

    fn mouse_down(&mut self, event: &MouseEvent) {
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
