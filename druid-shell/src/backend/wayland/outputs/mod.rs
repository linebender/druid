use wayland_client as wlc;
use wayland_client::protocol::wl_output;

use super::display;
use super::error;
mod wlr;
mod xdg;

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum Event {
    Located(Meta),
    Removed(Meta),
}

pub fn auto<'a>(
    registry: &'a wlc::GlobalManager,
) -> Result<calloop::channel::Channel<Event>, error::Error> {
    tracing::debug!("detecting wlr outputs");
    match wlr::detect(registry) {
        Ok(rx) => return Ok(rx),
        Err(cause) => tracing::info!("unable to detect wlr outputs {:?}", cause),
    }

    tracing::debug!("detecting xdg outputs");
    match xdg::detect(registry) {
        Ok(rx) => return Ok(rx),
        Err(cause) => tracing::info!("unable to detect xdg outputs {:?}", cause),
    }

    Err(error::Error::string("unable to detect display outputs"))
}

pub(super) fn current<'a>(env: &'a display::Environment) -> Result<Vec<Meta>, error::Error> {
    let rx = auto(&env.registry)?;
    let mut cache = std::collections::BTreeMap::new();
    let mut eventloop: calloop::EventLoop<(
        calloop::LoopSignal,
        &mut std::collections::BTreeMap<String, Meta>,
    )> = calloop::EventLoop::try_new().expect("failed to initialize the displays event loop!");
    let signal = eventloop.get_signal();
    let handle = eventloop.handle();
    handle
        .insert_source(rx, {
            move |event, _ignored, (signal, cache)| {
                let event = match event {
                    calloop::channel::Event::Msg(event) => event,
                    calloop::channel::Event::Closed => return signal.stop(),
                };

                match event {
                    Event::Located(meta) => {
                        cache.insert(meta.name.clone(), meta.clone());
                    }
                    Event::Removed(meta) => {
                        cache.remove(&meta.name);
                    }
                }
            }
        })
        .map_err(error::Error::error)?;

    // do a round trip to flush commands.
    let mut queue = env.queue.try_borrow_mut().map_err(error::Error::error)?;
    queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .map_err(error::Error::error)?;

    let expected = display::count(&env.registry, "wl_output");
    let result: std::sync::Arc<std::cell::RefCell<Vec<Meta>>> =
        std::sync::Arc::new(std::cell::RefCell::new(Vec::new()));
    eventloop
        .run(
            std::time::Duration::from_secs(1),
            &mut (signal, &mut cache),
            {
                let result = result.clone();
                move |(signal, cache)| {
                    if expected <= cache.len() {
                        result.replace(cache.values().cloned().collect());
                        signal.stop();
                        return;
                    }
                }
            },
        )
        .map_err(error::Error::error)?;
    Ok(result.take())
}

pub trait Wayland {
    fn consume<'a>(
        &'a mut self,
        obj: &'a wlc::Main<wl_output::WlOutput>,
        event: &'a wl_output::Event,
    );
}

#[derive(Clone, Debug)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

impl Default for Dimensions {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }
}

impl From<(i32, i32)> for Dimensions {
    fn from(v: (i32, i32)) -> Self {
        Self {
            width: v.0,
            height: v.1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Default for Position {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl From<(i32, i32)> for Position {
    fn from(v: (i32, i32)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}

#[derive(Debug, Clone)]
pub struct Mode {
    pub logical: Dimensions,
    pub refresh: i32,
    pub preferred: bool,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            logical: Default::default(),
            refresh: 0,
            preferred: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Meta {
    pub name: String,
    pub description: String,
    pub logical: Dimensions,
    pub refresh: i32,
    pub physical: Dimensions,
    pub subpixel: wl_output::Subpixel,
    pub transform: wl_output::Transform,
    pub make: String,
    pub model: String,
    pub scale: f64,
    pub enabled: bool,
    pub position: Position,
}

impl Meta {
    pub fn normalize(mut self) -> Self {
        match self.transform {
            wl_output::Transform::Flipped270 | wl_output::Transform::_270 => {
                self.logical = Dimensions::from((self.logical.height, self.logical.width));
                self.physical = Dimensions::from((self.physical.height, self.physical.width));
            }
            _ => {}
        }
        self
    }
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            name: Default::default(),
            description: Default::default(),
            logical: Default::default(),
            refresh: Default::default(),
            physical: Default::default(),
            position: Default::default(),
            subpixel: wl_output::Subpixel::Unknown,
            transform: wl_output::Transform::Normal,
            make: Default::default(),
            model: Default::default(),
            scale: Default::default(),
            enabled: Default::default(),
        }
    }
}
