use druid::{
    widget::{Button, Controller, Flex, Label, TextBox},
    AppLauncher, Data, Env, Event, EventCtx, Lens, Prism, WidgetExt, WindowDesc,
};
use druid::{PlatformError, Selector, Widget};

const LOGIN: Selector<MainState> = Selector::new("druid-enums.basic.login");

#[derive(Clone, Data, Prism)]
enum AppState {
    Login(LoginState),
    Main(MainState),
}

#[derive(Clone, Data, Lens, Default)]
struct LoginState {
    user: String,
}

#[derive(Clone, Data, Lens)]
struct MainState {
    user: String,
    count: u32,
}

fn main() -> Result<(), PlatformError> {
    let window = WindowDesc::new(ui).title("Druid Enums");
    let state = AppState::Login(LoginState::default());
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(state)
}

pub struct Container<S, A1, A2, P1, P2, W1, W2> {
    w1: druid::prism::PrismWrap<A1, P1, W1>,
    w2: druid::prism::PrismWrap<A2, P2, W2>,
    _marker: std::marker::PhantomData<S>,
}

impl<S, A1, A2, P1, P2, W1, W2> Container<S, A1, A2, P1, P2, W1, W2> {
    pub fn new(
        w1: druid::prism::PrismWrap<A1, P1, W1>,
        w2: druid::prism::PrismWrap<A2, P2, W2>,
    ) -> Self {
        Self {
            w1,
            w2,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, A1, A2, P1, P2, W1, W2> Widget<S> for Container<S, A1, A2, P1, P2, W1, W2>
where
    S: Data,
    A1: Data,
    A2: Data,
    P1: Prism<S, A1>,
    P2: Prism<S, A2>,
    W1: Widget<A1>,
    W2: Widget<A2>,
{
    fn event(
        &mut self,
        ctx: &mut ::druid::EventCtx,
        event: &::druid::Event,
        data: &mut S,
        env: &::druid::Env,
    ) {
        self.w1.event(ctx, event, data, env);
        self.w2.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut ::druid::LifeCycleCtx,
        event: &::druid::LifeCycle,
        data: &S,
        env: &::druid::Env,
    ) {
        self.w1.lifecycle(ctx, event, data, env);
        self.w2.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut ::druid::UpdateCtx, old_data: &S, data: &S, env: &::druid::Env) {
        self.w1.update(ctx, old_data, data, env);
        self.w2.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut ::druid::LayoutCtx,
        bc: &::druid::BoxConstraints,
        data: &S,
        env: &::druid::Env,
    ) -> ::druid::Size {
        self.w1.layout(ctx, bc, data, env) + self.w2.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut ::druid::PaintCtx, data: &S, env: &::druid::Env) {
        self.w1.paint(ctx, data, env);
        self.w2.paint(ctx, data, env);
    }
}

fn ui() -> impl Widget<AppState> {
    Container::new(
        login_ui().prism(AppState::login),
        main_ui().prism(AppState::main),
    )
    // .debug_paint_layout()
    .controller(LoginController)
}

fn login_ui() -> impl Widget<LoginState> {
    fn login(ctx: &mut EventCtx, state: &mut LoginState, _: &Env) {
        ctx.submit_command(LOGIN.with(MainState::from(state.clone())), None)
    }

    Flex::row()
        .with_child(TextBox::new().lens(LoginState::user))
        .with_spacer(15.0)
        .with_child(Button::new("Login").on_click(login))
        .center()
    // .debug_paint_layout()
}

fn main_ui() -> impl Widget<MainState> {
    Flex::column()
        .with_child(Label::dynamic(MainState::welcome_label))
        .with_spacer(5.0)
        .with_child(
            Button::dynamic(MainState::count_label)
                .on_click(|_, state: &mut MainState, _| state.count += 1),
        )
        .center()
    // .debug_paint_layout()
}

struct LoginController;
impl<W: Widget<AppState>> Controller<AppState, W> for LoginController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(LOGIN) => {
                let main_state = cmd.get_unchecked(LOGIN).clone();
                *data = AppState::Main(main_state);
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}

impl MainState {
    pub fn welcome_label(&self, _: &Env) -> String {
        format!("Welcome {}!", self.user)
    }

    pub fn count_label(&self, _: &Env) -> String {
        format!("clicked {} times", self.count)
    }
}

impl From<LoginState> for MainState {
    fn from(login: LoginState) -> Self {
        MainState {
            user: login.user,
            count: 0,
        }
    }
}
