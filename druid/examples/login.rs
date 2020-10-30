// adapted from https://github.com/Finnerale/druid-enums/blob/master/examples/login.rs

use druid::{
    widget::{overlay::Overlay2, Button, Controller, Flex, Label, TextBox},
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

fn ui() -> impl Widget<AppState> {
    Overlay2::new(
        login_ui().prism(AppState::login).padding(0.2),
        main_ui().prism(AppState::main),
    )
    // .debug_paint_layout()
    .controller(LoginController)
}

fn login_ui() -> impl Widget<LoginState> {
    fn login(ctx: &mut EventCtx, state: &mut LoginState, _: &Env) {
        ctx.submit_command(LOGIN.with(MainState::from(state.clone())))
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
