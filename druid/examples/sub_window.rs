// Copyright 2021 The Druid Authors.
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

use druid::commands::CLOSE_WINDOW;
use druid::lens::Unit;
use druid::widget::{
    Align, Button, Checkbox, Controller, ControllerHost, EnvScope, Flex, Label, TextBox,
};
use druid::{
    theme, Affine, AppLauncher, BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, Lens,
    LensExt, LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, Point, Rect, RenderContext, Size,
    TimerToken, UpdateCtx, Widget, WidgetExt, WindowConfig, WindowDesc, WindowId, WindowSizePolicy,
};
use druid_shell::piet::Text;
use druid_shell::{Screen, WindowLevel};
use instant::{Duration, Instant};
use piet_common::{TextLayout, TextLayoutBuilder};

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const TEXT_BOX_WIDTH: f64 = 200.0;
const WINDOW_TITLE: LocalizedString<HelloState> = LocalizedString::new("Hello World!");

#[derive(Clone, Data, Lens)]
struct SubState {
    my_stuff: String,
}

#[derive(Clone, Data, Lens)]
struct HelloState {
    name: String,
    sub: SubState,
    closeable: bool,
}

pub fn main() {
    // describe the main window
    let main_window = WindowDesc::new(build_root_widget())
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = HelloState {
        name: "World".into(),
        sub: SubState {
            my_stuff: "It's mine!".into(),
        },
        closeable: true,
    };

    // start the application
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(initial_state)
        .expect("Failed to launch application");
}

enum TooltipState {
    Showing(WindowId),
    Waiting {
        last_move: Instant,
        timer_expire: Instant,
        token: TimerToken,
        window_pos: Point,
    },
    Fresh,
}

struct TooltipController {
    tip: String,
    state: TooltipState,
}

impl TooltipController {
    pub fn new(tip: impl Into<String>) -> Self {
        TooltipController {
            tip: tip.into(),
            state: TooltipState::Fresh,
        }
    }
}

impl<T, W: Widget<T>> Controller<T, W> for TooltipController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        let wait_duration = Duration::from_millis(500);
        let resched_dur = Duration::from_millis(50);
        let cursor_size = Size::new(15., 15.);
        let now = Instant::now();
        let new_state = match &self.state {
            TooltipState::Fresh => match event {
                Event::MouseMove(me) if ctx.is_hot() => Some(TooltipState::Waiting {
                    last_move: now,
                    timer_expire: now + wait_duration,
                    token: ctx.request_timer(wait_duration),
                    window_pos: me.window_pos,
                }),
                _ => None,
            },
            TooltipState::Waiting {
                last_move,
                timer_expire,
                token,
                window_pos,
            } => match event {
                Event::MouseMove(me) if ctx.is_hot() => {
                    let (cur_token, cur_expire) = if *timer_expire - now < resched_dur {
                        (ctx.request_timer(wait_duration), now + wait_duration)
                    } else {
                        (*token, *timer_expire)
                    };
                    Some(TooltipState::Waiting {
                        last_move: now,
                        timer_expire: cur_expire,
                        token: cur_token,
                        window_pos: me.window_pos,
                    })
                }
                Event::Timer(tok) if tok == token => {
                    let deadline = *last_move + wait_duration;
                    ctx.set_handled();
                    if deadline > now {
                        let wait_for = deadline - now;
                        tracing::info!("Waiting another {:?}", wait_for);
                        Some(TooltipState::Waiting {
                            last_move: *last_move,
                            timer_expire: deadline,
                            token: ctx.request_timer(wait_for),
                            window_pos: *window_pos,
                        })
                    } else {
                        let win_id = ctx.new_sub_window(
                            WindowConfig::default()
                                .show_titlebar(false)
                                .window_size_policy(WindowSizePolicy::Content)
                                .set_level(WindowLevel::Tooltip)
                                .set_position(
                                    ctx.window().get_position()
                                        + window_pos.to_vec2()
                                        + cursor_size.to_vec2(),
                                ),
                            Label::<()>::new(self.tip.clone()),
                            (),
                            env.clone(),
                        );
                        Some(TooltipState::Showing(win_id))
                    }
                }
                _ => None,
            },
            TooltipState::Showing(win_id) => {
                match event {
                    Event::MouseMove(me) if !ctx.is_hot() => {
                        // TODO another timer on leaving
                        tracing::info!("Sending close window for {:?}", win_id);
                        ctx.submit_command(CLOSE_WINDOW.to(*win_id));
                        Some(TooltipState::Waiting {
                            last_move: now,
                            timer_expire: now + wait_duration,
                            token: ctx.request_timer(wait_duration),
                            window_pos: me.window_pos,
                        })
                    }
                    _ => None,
                }
            }
        };

        if let Some(state) = new_state {
            self.state = state;
        }

        if !ctx.is_handled() {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &T,
        env: &Env,
    ) {
        if let LifeCycle::HotChanged(false) = event {
            if let TooltipState::Showing(win_id) = self.state {
                ctx.submit_command(CLOSE_WINDOW.to(win_id));
            }
            self.state = TooltipState::Fresh;
        }
        child.lifecycle(ctx, event, data, env)
    }
}

struct DragWindowController {
    init_pos: Option<Point>,
    //dragging: bool
}

impl DragWindowController {
    pub fn new() -> Self {
        DragWindowController { init_pos: None }
    }
}

impl<T, W: Widget<T>> Controller<T, W> for DragWindowController {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(me) if me.buttons.has_left() => {
                ctx.set_active(true);
                self.init_pos = Some(me.window_pos)
            }
            Event::MouseMove(me) if ctx.is_active() && me.buttons.has_left() => {
                if let Some(init_pos) = self.init_pos {
                    let within_window_change = me.window_pos.to_vec2() - init_pos.to_vec2();
                    let old_pos = ctx.window().get_position();
                    let new_pos = old_pos + within_window_change;
                    tracing::info!(
                        "Drag {:?} ",
                        (
                            init_pos,
                            me.window_pos,
                            within_window_change,
                            old_pos,
                            new_pos
                        )
                    );
                    ctx.window().set_position(new_pos)
                }
            }
            Event::MouseUp(_me) if ctx.is_active() => {
                self.init_pos = None;
                ctx.set_active(false)
            }
            _ => (),
        }
        child.event(ctx, event, data, env)
    }
}

struct ScreenThing;

impl Widget<()> for ScreenThing {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut (), _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &(), _env: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &(), _data: &(), _env: &Env) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &(),
        _env: &Env,
    ) -> Size {
        bc.constrain(Size::new(800.0, 600.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &(), env: &Env) {
        let sz = ctx.size();

        let monitors = Screen::get_monitors();
        let all = monitors
            .iter()
            .map(|x| x.virtual_rect())
            .fold(Rect::ZERO, |s, r| r.union(s));
        if all.width() > 0. && all.height() > 0. {
            let trans = Affine::scale(f64::min(sz.width / all.width(), sz.height / all.height()))
                * Affine::translate(all.origin().to_vec2()).inverse();
            let font = env.get(theme::UI_FONT).family;

            for (i, mon) in monitors.iter().enumerate() {
                let vr = mon.virtual_rect();
                let tr = trans.transform_rect_bbox(vr);
                ctx.stroke(tr, &Color::WHITE, 1.0);

                if let Ok(tl) = ctx
                    .text()
                    .new_text_layout(format!(
                        "{}:{}x{}@{},{}",
                        i,
                        vr.width(),
                        vr.height(),
                        vr.x0,
                        vr.y0
                    ))
                    .max_width(tr.width() - 5.)
                    .font(font.clone(), env.get(theme::TEXT_SIZE_NORMAL))
                    .text_color(Color::WHITE)
                    .build()
                {
                    ctx.draw_text(&tl, tr.center() - tl.size().to_vec2() * 0.5);
                }
            }
        }
    }
}

struct CancelClose;

impl<W: Widget<bool>> Controller<bool, W> for CancelClose {
    fn event(
        &mut self,
        w: &mut W,
        ctx: &mut EventCtx<'_, '_>,
        event: &Event,
        data: &mut bool,
        env: &Env,
    ) {
        match (&data, event) {
            (false, Event::WindowCloseRequested) => ctx.set_handled(),
            _ => w.event(ctx, event, data, env),
        }
    }
}

fn build_root_widget() -> impl Widget<HelloState> {
    let label = EnvScope::new(
        |env, _t| env.set(theme::LABEL_COLOR, env.get(theme::PRIMARY_LIGHT)),
        ControllerHost::new(
            Label::new(|data: &HelloState, _env: &Env| {
                format!("Hello {}! {} ", data.name, data.sub.my_stuff)
            }),
            TooltipController::new("Tips! Are good"),
        ),
    );
    // a textbox that modifies `name`.
    let textbox = TextBox::new()
        .with_placeholder("Who are we greeting?")
        .fix_width(TEXT_BOX_WIDTH)
        .lens(HelloState::sub.then(SubState::my_stuff));

    let button = Button::new("Make sub window")
        .on_click(|ctx, data: &mut SubState, env| {
            let tb = TextBox::new().lens(SubState::my_stuff);
            let drag_thing = Label::new("Drag me").controller(DragWindowController::new());
            let col = Flex::column().with_child(drag_thing).with_child(tb);

            ctx.new_sub_window(
                WindowConfig::default()
                    .show_titlebar(false)
                    .window_size(Size::new(100., 100.))
                    .set_position(Point::new(1000.0, 500.0))
                    .set_level(WindowLevel::AppWindow),
                col,
                data.clone(),
                env.clone(),
            );
        })
        .center()
        .lens(HelloState::sub);

    let check_box =
        ControllerHost::new(Checkbox::new("Closeable?"), CancelClose).lens(HelloState::closeable);
    // arrange the two widgets vertically, with some padding
    let layout = Flex::column()
        .with_child(label)
        .with_flex_child(ScreenThing.lens(Unit::default()).padding(5.), 1.)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(textbox)
        .with_child(button)
        .with_child(check_box);

    // center the two widgets in the available space
    Align::centered(layout)
}
