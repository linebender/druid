// Copyright 2022 the Druid Authors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::single_match)]
use super::error;
use std::collections::BTreeMap;
use wayland_client as wlc;
use wayland_client::protocol::wl_registry;
use wayland_protocols::xdg_shell::client::xdg_wm_base;

type GlobalEventConsumer = dyn Fn(&wlc::GlobalEvent, &wlc::Attached<wl_registry::WlRegistry>, &wlc::DispatchData)
    + 'static;

#[derive(Clone)]
pub struct GlobalEventSubscription {
    id: u64,
    sub: std::sync::Arc<GlobalEventConsumer>,
}

impl GlobalEventSubscription {
    fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }
}

impl GlobalEventSubscription {
    fn consume(
        &self,
        event: &wlc::GlobalEvent,
        registry: &wlc::Attached<wl_registry::WlRegistry>,
        ctx: &wlc::DispatchData,
    ) {
        (self.sub)(event, registry, ctx)
    }
}

impl<X> From<X> for GlobalEventSubscription
where
    X: Fn(&wlc::GlobalEvent, &wlc::Attached<wl_registry::WlRegistry>, &wlc::DispatchData) + 'static,
{
    fn from(closure: X) -> Self {
        Self {
            id: 0,
            sub: std::sync::Arc::new(closure),
        }
    }
}

pub trait GlobalEventDispatch {
    fn subscribe(&self, sub: impl Into<GlobalEventSubscription>) -> GlobalEventSubscription;
    fn release(&self, s: &GlobalEventSubscription);
}

pub(super) struct Dispatcher {
    incr: crate::Counter,
    subscriptions: std::cell::RefCell<BTreeMap<u64, GlobalEventSubscription>>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self {
            incr: crate::Counter::new(),
            subscriptions: std::cell::RefCell::new(BTreeMap::new()),
        }
    }
}

impl Dispatcher {
    fn consume(
        &self,
        event: &wlc::GlobalEvent,
        registry: &wlc::Attached<wl_registry::WlRegistry>,
        ctx: &wlc::DispatchData,
    ) {
        for (_, sub) in self.subscriptions.borrow().iter() {
            sub.consume(event, registry, ctx);
        }
    }
}

impl GlobalEventDispatch for Dispatcher {
    fn subscribe(&self, sub: impl Into<GlobalEventSubscription>) -> GlobalEventSubscription {
        let sub = sub.into().with_id(self.incr.next());
        self.subscriptions.borrow_mut().insert(sub.id, sub.clone());
        sub
    }

    fn release(&self, s: &GlobalEventSubscription) {
        self.subscriptions.borrow_mut().remove(&s.id);
    }
}

pub(super) struct Environment {
    pub(super) display: wlc::Display,
    pub(super) registry: wlc::GlobalManager,
    pub(super) xdg_base: wlc::Main<xdg_wm_base::XdgWmBase>,
    pub(super) queue: std::rc::Rc<std::cell::RefCell<wlc::EventQueue>>,
    dispatcher: std::sync::Arc<Dispatcher>,
}

impl GlobalEventDispatch for Environment {
    fn subscribe(&self, sub: impl Into<GlobalEventSubscription>) -> GlobalEventSubscription {
        self.dispatcher.subscribe(sub)
    }

    fn release(&self, s: &GlobalEventSubscription) {
        self.dispatcher.release(s)
    }
}

impl GlobalEventDispatch for std::sync::Arc<Environment> {
    fn subscribe(&self, sub: impl Into<GlobalEventSubscription>) -> GlobalEventSubscription {
        self.dispatcher.subscribe(sub)
    }

    fn release(&self, s: &GlobalEventSubscription) {
        self.dispatcher.release(s)
    }
}

pub(super) fn new(dispatcher: Dispatcher) -> Result<Environment, error::Error> {
    let dispatcher = std::sync::Arc::new(dispatcher);
    let d = wlc::Display::connect_to_env()?;

    let mut queue = d.create_event_queue();
    let handle = d.attach(queue.token());
    let registry = wlc::GlobalManager::new_with_cb(&handle, {
        let dispatcher = dispatcher.clone();
        move |event, registry, ctx| {
            dispatcher.consume(&event, &registry, &ctx);
        }
    });

    // do a round trip to make sure we have all the globals
    queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .map_err(error::Error::fatal)?;

    // 3 is the max version supported by wayland-rs 0.29.5
    let xdg_base = registry
        .instantiate_range::<xdg_wm_base::XdgWmBase>(1, 3)
        .map_err(|e| error::Error::global("xdg_wm_base", 1, e))?;

    // We do this to make sure wayland knows we're still responsive.
    //
    // NOTE: This means that clients mustn't hold up the event loop, or else wayland might kill
    // your app's connection. Move *everything* to another thread, including e.g. file i/o,
    // computation, network, ... This is good practice for all back-ends: it will improve
    // responsiveness.
    xdg_base.quick_assign(|xdg_base, event, ctx| {
        tracing::debug!(
            "global xdg_base events {:?} {:?} {:?}",
            xdg_base,
            event,
            ctx
        );
        match event {
            xdg_wm_base::Event::Ping { serial } => xdg_base.pong(serial),
            _ => (),
        }
    });

    let env = Environment {
        queue: std::rc::Rc::new(std::cell::RefCell::new(queue)),
        display: d,
        registry,
        xdg_base,
        dispatcher,
    };

    Ok(env)
}

#[allow(unused)]
pub(super) fn print(reg: &wlc::GlobalManager) {
    let mut globals_list = reg.list();
    globals_list.sort_by(|(_, name1, version1), (_, name2, version2)| {
        name1.cmp(name2).then(version1.cmp(version2))
    });

    for (id, name, version) in globals_list.into_iter() {
        tracing::debug!("{:?}@{:?} - {:?}", name, version, id);
    }
}

pub(super) fn count(reg: &wlc::GlobalManager, i: &str) -> usize {
    reg.list().iter().filter(|(_, name, _)| name == i).count()
}
