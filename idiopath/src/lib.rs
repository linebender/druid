use pyo3::prelude::*;

#[pyfunction]
fn ui(py_app: PyObject) -> PyResult<String> {
    /*
    Python::with_gil(|py| {
        let result = py_app.call(py, (), None).unwrap();
        let b: PyResult<PyRef<Button>> = result.as_ref(py).extract();
        if let Ok(button) = b {
            println!("button label is {}", button.label);
        }
    });
    */
    py_main(py_app);
    Ok("hello".to_string())
}

#[pyclass]
struct Button {
    label: String,
}

#[pyclass]
struct PyView {
    view: Box<dyn AnyView<(), ()> + Send>,
}

#[pyfunction]
fn button(label: String) -> PyView {
    let view = crate::view::button::Button::new(label, |_data| println!("clicked"));
    PyView {
        view: Box::new(view),
    }
}

#[pymethods]
impl Button {
    #[new]
    fn new(label: String) -> Self {
        println!("label is {}", label);
        Button { label }
    }
}

#[pymodule]
fn foo(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(ui, m)?)?;
    m.add_function(wrap_pyfunction!(button, m)?)?;
    m.add_class::<Button>()?;
    m.add_class::<PyView>()?;
    Ok(())
}

fn run_cycle(py_app: &PyAny, py: Python) -> impl View<(), (), Element = impl Widget> {
    let py_app = py_app.to_object(py);
    py_app.call(py, (), None).unwrap()
}

mod app;
mod event;
mod id;
mod view;
mod view_tuple;
mod widget;

use std::any::Any;

use app::App;
use druid_shell::kurbo::Size;
use druid_shell::piet::{Color, RenderContext};

use druid_shell::{
    Application, Cursor, HotKey, Menu, MouseEvent, Region, SysMods, WinHandler, WindowBuilder,
    WindowHandle,
};
use view::adapt::Adapt;
use view::any_view::AnyView;
use view::column::Column;
use view::memoize::Memoize;
use view::View;
use widget::{Widget, AnyWidget};

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);

struct MainState<T, V: View<T, ()>, F: FnMut(&mut T) -> V>
where
    V::Element: Widget,
{
    size: Size,
    handle: WindowHandle,
    app: App<T, V, F>,
}

impl<T: 'static, V: View<T, ()> + 'static, F: FnMut(&mut T) -> V + 'static> WinHandler
    for MainState<T, V, F>
where
    V::Element: Widget,
{
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut druid_shell::piet::Piet, _: &Region) {
        let rect = self.size.to_rect();
        piet.fill(rect, &BG_COLOR);
        self.app.paint(piet);
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => {
                self.handle.close();
                Application::global().quit()
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn mouse_move(&mut self, _event: &MouseEvent) {
        self.handle.set_cursor(&Cursor::Arrow);
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        self.app.mouse_down(event.pos);
        self.handle.invalidate();
    }

    fn mouse_up(&mut self, _event: &MouseEvent) {}

    fn size(&mut self, size: Size) {
        self.size = size;
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

impl<T, V: View<T, ()>, F: FnMut(&mut T) -> V> MainState<T, V, F>
where
    V::Element: Widget,
{
    fn new(app: App<T, V, F>) -> Self {
        let state = MainState {
            size: Default::default(),
            handle: Default::default(),
            app,
        };
        state
    }
}

fn py_main(py_app: PyObject) {
    //tracing_subscriber::fmt().init();
    let mut file_menu = Menu::new();
    file_menu.add_item(
        0x100,
        "E&xit",
        Some(&HotKey::new(SysMods::Cmd, "q")),
        true,
        false,
    );
    let mut menubar = Menu::new();
    menubar.add_dropdown(Menu::new(), "Application", true);
    menubar.add_dropdown(file_menu, "&File", true);

    let app = App::new((), move |_| {
        Python::with_gil(|py| run_cycle(py_app.as_ref(py), py))
    });
    let druid_app = Application::new().unwrap();
    let mut builder = WindowBuilder::new(druid_app.clone());
    let main_state = MainState::new(app);
    builder.set_handler(Box::new(main_state));
    builder.set_title("Idiopath");
    builder.set_menu(menubar);

    let window = builder.build().unwrap();
    window.show();

    druid_app.run(None);
}

// idiopath trait implementations on Python objects

impl View<(), ()> for PyObject {
    type State = Box<dyn Any>;

    type Element = Box<dyn AnyWidget>;

    fn build(&self, id_path: &mut id::IdPath) -> (id::Id, Self::State, Self::Element) {
        Python::with_gil(|py| {
            let py_view: PyRef<PyView> = self.as_ref(py).extract().unwrap();
            py_view.view.build(id_path)
        })
    }

    fn rebuild(
        &self,
        id_path: &mut id::IdPath,
        prev: &Self,
        id: &mut id::Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) {
        Python::with_gil(|py| {
            let py_view: PyRef<PyView> = self.as_ref(py).extract().unwrap();
            let prev: PyRef<PyView> = prev.as_ref(py).extract().unwrap();
            py_view.view.rebuild(id_path, &prev.view, id, state, element)
        })
    }

    fn event(
        &self,
        id_path: &[id::Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut (),
    ) -> () {
        Python::with_gil(|py| {
            let py_view: PyRef<PyView> = self.as_ref(py).extract().unwrap();
            py_view.view.event(id_path, state, event, app_state);
        })
    }
}