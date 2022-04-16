// Copyright 2022 The Druid Authors.
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

use id::{Id, IdPath};
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use view::button::Button;
use view_tuple::ViewTuple;

#[pyfunction]
fn ui(init_state: PyObject, py_app: PyObject) -> PyResult<String> {
    py_main(init_state, py_app);
    Ok("hello".to_string())
}

#[pyclass]
struct PyView {
    view: Box<dyn AnyView<PyObject, PyObject> + Send>,
}

#[pyfunction]
fn button(label: String, callback: PyObject) -> PyView {
    let view = Button::new(label, move |data: &mut PyObject| {
        println!("clicked");
        Python::with_gil(|py| callback.call1(py, (&*data,)).unwrap().to_object(py))
    });
    PyView {
        view: Box::new(view),
    }
}

#[pyfunction(children = "*")]
fn column(children: &PyTuple) -> PyView {
    // Note: maybe better to type this as PyTuple instead of PyObject, and
    // avoid downcasting in the ViewTuple impl.
    let children = Python::with_gil(|py| children.to_object(py));
    let view = Column::new(children);
    PyView { view: Box::new(view) }
}

#[pymodule]
fn foo(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(ui, m)?)?;
    m.add_function(wrap_pyfunction!(button, m)?)?;
    m.add_function(wrap_pyfunction!(column, m)?)?;
    m.add_class::<PyView>()?;
    Ok(())
}

fn run_cycle(
    py_app: &PyAny,
    data: &PyAny,
    py: Python,
) -> impl View<PyObject, PyObject, Element = impl Widget> {
    let py_app = py_app.to_object(py);
    py_app.call1(py, (data,)).unwrap()
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
use widget::{AnyWidget, Widget};

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

fn py_main(init_state: PyObject, py_app: PyObject) {
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

    let app = App::new(init_state, move |data| {
        let py_view_tree = Python::with_gil(|py| run_cycle(py_app.as_ref(py), data.as_ref(py), py));
        Adapt::new(
            |data: &mut PyObject, child| {
                child.call(data);
            },
            py_view_tree,
        )
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

impl View<PyObject, PyObject> for PyObject {
    type State = Box<dyn Any>;

    type Element = Box<dyn AnyWidget>;

    fn build(&self, id_path: &mut IdPath) -> (Id, Self::State, Self::Element) {
        // Note: the View trait will probably grow a context, which would also be
        // a good place to hold the GIL, so we don't need to acquire it every time.
        Python::with_gil(|py| {
            let py_view: PyRef<PyView> = self.as_ref(py).extract().unwrap();
            py_view.view.build(id_path)
        })
    }

    fn rebuild(
        &self,
        id_path: &mut IdPath,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) {
        Python::with_gil(|py| {
            let py_view: PyRef<PyView> = self.as_ref(py).extract().unwrap();
            let prev: PyRef<PyView> = prev.as_ref(py).extract().unwrap();
            py_view
                .view
                .rebuild(id_path, &prev.view, id, state, element)
        })
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut PyObject,
    ) -> PyObject {
        Python::with_gil(|py| {
            let py_view: PyRef<PyView> = self.as_ref(py).extract().unwrap();
            py_view.view.event(id_path, state, event, app_state)
        })
    }
}

impl ViewTuple<PyObject, PyObject> for PyObject {
    type State = Vec<(Id, Box<dyn Any>)>;

    type Elements = Vec<Box<dyn AnyWidget>>;

    fn build(&self, id_path: &mut IdPath) -> (Self::State, Self::Elements) {
        Python::with_gil(|py| {
            let py_tuple: &PyTuple = self.as_ref(py).downcast().unwrap();
            let n = py_tuple.len();
            let mut state = Vec::with_capacity(n);
            let mut elements = Vec::with_capacity(n);
            for child in py_tuple {
                let (id, child_state, child_view) = View::build(&child.to_object(py), id_path);
                state.push((id, child_state));
                elements.push(child_view);
            }
            (state, elements)
        })
    }

    fn rebuild(
        &self,
        id_path: &mut IdPath,
        prev: &Self,
        state: &mut Self::State,
        els: &mut Self::Elements,
    ) {
        Python::with_gil(|py| {
            let py_tuple: &PyTuple = self.as_ref(py).downcast().unwrap();
            let prev_tuple: &PyTuple = prev.as_ref(py).downcast().unwrap();
            // Note: we're not dealing with the tuple changing size, but we could.
            for (i, child) in py_tuple.iter().enumerate() {
                let (child_id, child_state) = &mut state[i];
                View::rebuild(
                    &child.to_object(py),
                    id_path,
                    &prev_tuple.get_item(i).unwrap().to_object(py),
                    child_id,
                    child_state,
                    &mut els[i],
                );
            }
        });
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        event: Box<dyn Any>,
        app_state: &mut PyObject,
    ) -> PyObject {
        let hd = id_path[0];
        let tl = &id_path[1..];
        let mut found = None;
        Python::with_gil(|py| {
            let py_tuple: &PyTuple = self.as_ref(py).downcast().unwrap();
            for (i, child) in py_tuple.iter().enumerate() {
                if hd == state[i].0 {
                    found = Some((i, child.to_object(py)));
                    break;
                }
            }
        });
        // We have to release the GIL here to avoid double-free of event (could
        // also be solved by interior mutability). Interesting question: should
        // we have the same pattern for the other methods?
        //
        // How expensive is it to constantly re-acquire the GIL?
        if let Some((i, child)) = found {
            View::event(&child, tl, &mut state[i].1, event, app_state)            
        } else {
            panic!("child of Python ViewTuple not found");
        }
    }
}
