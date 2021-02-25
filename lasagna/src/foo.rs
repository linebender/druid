// scratchpad for downcasting

use std::any::Any;

trait Widget: Element {
    fn do_widget(&self) {
        println!("doing widget stuff");
    }
}

trait Element {
    fn do_element(&self) {
        println!("doing element stuff");
    }
}

struct Container {
    inner: Box<dyn Widget + 'static>,
}

struct Leaf;

impl Container {
    fn from_any(a: Box<dyn Any + 'static>) -> Container {
        Container {
            inner: *a.downcast::<Box<dyn Widget>>().unwrap(),
        }
    }
}

impl Element for Container {
    fn do_element(&self) {
        println!("element for container");
        self.inner.do_element();
    }
}

impl Widget for Container {
    fn do_widget(&self) {
        println!("widget for container");
        self.inner.do_widget();
    }
}

impl Element for Leaf {}

impl Widget for Leaf {}

pub fn foo() {
    let l: Box<dyn Widget> = Box::new(Leaf);
    let c = Container::from_any(Box::new(l));
    c.do_element();
    c.do_widget();
}

