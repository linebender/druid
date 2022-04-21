use crate::{Widget, widget::RawEvent};
use std::{any::Any, marker::PhantomData};

use crate::{
    id::{Id},
};

pub trait MaybeFrom<T>: Sized {
    fn maybe_from(value: &T) -> Option<Self>;
}

pub trait Controller: Default {
    type Event: MaybeFrom<RawEvent>;
}
pub struct Interactive<C: Controller>{
    phantom: PhantomData<C>,
}

impl<C: Controller> Interactive<C> {
    pub fn new<T, A, V: View<T, A>>(child: V, handler: impl Fn(&C::Event, &mut C, &mut T) -> crate::event::EventResult<A> + 'static) -> InteractiveView<C, C::Event, T, A, V> {
        InteractiveView::new(child, handler)
    }
}

pub struct InteractiveView<C, E, T, A, V: View<T, A>> {
    child: V,
    // phantom: PhantomData<(T, A)>,
    handler: Box<dyn Fn(&E, &mut C, &mut T) -> crate::event::EventResult<A>>,
}

impl<C, E, T, A, V: View<T, A>> InteractiveView<C, E, T, A, V>  {
    pub fn new(child: V, handler: impl Fn(&E, &mut C, &mut T) -> crate::event::EventResult<A> + 'static) -> Self {
        InteractiveView { child, handler: Box::new(handler) }
    }
}

use super::View;

impl<C: Default, E: 'static + MaybeFrom<RawEvent>, T, A, V: View<T, A>> View<T,A> for InteractiveView<C, E, T, A, V> 
where
    V::Element: Widget,
{
    type State = (C, V::State);

    type Element = crate::widget::interactive::Interactive<V::Element, E>;

    
    fn build(&self, cx: &mut super::Cx) -> (Id, Self::State, Self::Element) {
        let (id, (state, interactive)) = cx.with_new_id(|cx| {
            let (innerId, state, element) = self.child.build(cx);
            let interactive = crate::widget::interactive::Interactive::new(cx.id_path(), element);

            ((Default::default(), state), interactive)
        });
        
        (id, state, interactive)
    }


    fn rebuild(
        &self,
        cx: &mut super::Cx,
        prev: &Self,
        id: &mut Id,
        state: &mut Self::State,
        element: &mut Self::Element,
    ) -> bool {
        cx.with_id(*id, |cx| {
            self.child
            .rebuild(cx, &prev.child, id, &mut state.1, element.child_mut())
        })
    }

    fn event(
            &self,
            id_path: &[Id],
            state: &mut Self::State,
            event: Box<dyn Any>,
            app_state: &mut T,
        ) -> crate::event::EventResult<A> {
        (self.handler)(&event.downcast::<E>().expect("THERE SHOULD BE AN E IN HERE"), &mut state.0, app_state)
    }
}

pub trait InteractiveExt<T, A, V: View<T, A>> {
    fn handle_events_with_controller<C: Controller>(self, handler: impl Fn(&C::Event, &mut C, &mut T) -> crate::event::EventResult<A> + 'static) -> InteractiveView<C, C::Event, T, A, V>;

    fn handle_events<E>(self, handler: impl Fn(&E, &mut T) -> crate::event::EventResult<A> + 'static) -> InteractiveView<(), E, T, A, V>;

    fn handle_event<E, H: Fn(&mut T) -> crate::event::EventResult<A> + 'static>(self, handler: H ) -> InteractiveView<(), E, T, A, V>;
}

impl<T, A, V: View<T, A>> InteractiveExt<T, A, V> for V {
    fn handle_events_with_controller<C: Controller>(self, handler: impl Fn(&C::Event, &mut C, &mut T) -> crate::event::EventResult<A> + 'static) -> InteractiveView<C, C::Event, T, A, V> {
        InteractiveView::new(self, handler)
    }

    fn handle_events<E>(self, handler: impl Fn(&E, &mut T) -> crate::event::EventResult<A> + 'static) -> InteractiveView<(), E, T, A, V> {
        InteractiveView::new(self, move |event, _, data| {
            handler(event, data)
        })
    }

    fn handle_event<E, H : Fn(&mut T) -> crate::event::EventResult<A> + 'static>(self, handler: H ) -> InteractiveView<(), E, T, A, V>{
        InteractiveView::new(self, move |_, _, data| {
            handler(data)
        })
    }
}

pub enum ClickEvents {
    LeftClick,
    MiddleClick,
    RightClick
}

impl MaybeFrom<RawEvent> for ClickEvents {
    fn maybe_from(value: &RawEvent) -> Option<Self> {
        match value {
            RawEvent::MouseUp(event) => if event.button.is_left() {
                Some(ClickEvents::LeftClick)
            } else {
                None
            }
            _ => None
        }
        
    }
}

#[derive(Default)]
pub struct ClickController {}

impl Controller for ClickController {
    type Event = ClickEvents;
}

pub struct LeftClick {}

impl MaybeFrom<RawEvent> for LeftClick {
    fn maybe_from(value: &RawEvent) -> Option<Self> {
        match value {
            RawEvent::MouseUp(event) => if event.button.is_left() {
                Some(LeftClick{})
            } else {
                None
            }
            _ => None
        }
    }
}
