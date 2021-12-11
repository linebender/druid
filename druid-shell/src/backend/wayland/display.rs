#![allow(clippy::single_match)]
use super::error;
use std::collections::BTreeMap;
use wayland_client as wlc;
use wayland_client::protocol::wl_registry;
use wayland_protocols::xdg_shell::client::xdg_wm_base;

#[derive(Clone)]
pub struct GlobalEventSubscription {
    id: u64,
    sub: std::sync::Arc<dyn GlobalEventConsumer>,
}

impl GlobalEventSubscription {
    fn with_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }
}

impl GlobalEventConsumer for GlobalEventSubscription {
    fn consume(
        &self,
        event: &wlc::GlobalEvent,
        registry: &wlc::Attached<wl_registry::WlRegistry>,
        ctx: &wlc::DispatchData,
    ) {
        self.sub.consume(event, registry, ctx)
    }
}

impl<X> From<X> for GlobalEventSubscription
where
    X: Fn(&wlc::GlobalEvent, &wlc::Attached<wl_registry::WlRegistry>, &wlc::DispatchData)
        + GlobalEventConsumer
        + 'static,
{
    fn from(closure: X) -> Self {
        Self {
            id: 0,
            sub: std::sync::Arc::new(closure),
        }
    }
}

impl<X> GlobalEventConsumer for X
where
    X: Fn(&wlc::GlobalEvent, &wlc::Attached<wl_registry::WlRegistry>, &wlc::DispatchData) + 'static,
{
    fn consume(
        &self,
        event: &wlc::GlobalEvent,
        registry: &wlc::Attached<wl_registry::WlRegistry>,
        ctx: &wlc::DispatchData,
    ) {
        self(event, registry, ctx)
    }
}

pub trait GlobalEventDispatch {
    fn subscribe(&self, sub: impl Into<GlobalEventSubscription>) -> GlobalEventSubscription;
    fn release(&self, s: &GlobalEventSubscription);
}

pub trait GlobalEventConsumer {
    fn consume(
        &self,
        event: &wlc::GlobalEvent,
        registry: &wlc::Attached<wl_registry::WlRegistry>,
        ctx: &wlc::DispatchData,
    );
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

impl GlobalEventConsumer for Dispatcher {
    fn consume(
        &self,
        event: &wlc::GlobalEvent,
        registry: &wlc::Attached<wl_registry::WlRegistry>,
        ctx: &wlc::DispatchData,
    ) {
        // tracing::info!("global event initiated {:?} {:?}", registry, event);
        for (_, sub) in self.subscriptions.borrow().iter() {
            sub.consume(event, registry, ctx);
        }
        // tracing::info!("global event completed {:?} {:?}", registry, event);
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

// because we have the global environment we need to mark these as safe/send.
// strictly speaking we should probably guard the access to the various fields
// behind a mutex, but in practice we are not actually accessing across threads.
unsafe impl Sync for Environment {}
unsafe impl Send for Environment {}

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

pub(super) fn new(dispatcher: Dispatcher) -> Result<std::sync::Arc<Environment>, error::Error> {
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

    let xdg_base = registry
        .instantiate_exact::<xdg_wm_base::XdgWmBase>(2)
        .map_err(|e| error::Error::global("xdg_wm_base", 2, e))?;

    // We do this to make sure wayland knows we're still responsive.
    //
    // NOTE: This means that clients mustn't hold up the event loop, or else wayland might kill
    // your app's connection. Move *everything* to another thread, including e.g. file i/o,
    // computation, network, ... This is good practice for all back-ends: it will improve
    // responsiveness.
    xdg_base.quick_assign(|xdg_base, event, ctx| {
        tracing::info!(
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

    let env = std::sync::Arc::new(Environment {
        queue: std::rc::Rc::new(std::cell::RefCell::new(queue)),
        display: d,
        registry,
        xdg_base,
        dispatcher,
    });

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
