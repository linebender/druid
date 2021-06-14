use crate::kurbo::Size;

use druid_shell::{Application, Error as PlatformError, WindowBuilder};

use crate::shell_handler::ShellHandler;
use crate::Widget;

pub struct LaunchCtx;

pub trait App {
    type Action;
    fn update(&mut self, actions: &[Self::Action], update: &mut bool);
    // you will build and add windows to the 'ctx'?
    fn launch(&mut self, ctx: &mut LaunchCtx) -> Box<dyn Widget<Action = Self::Action>>;
}

pub fn launch<T: 'static>(app: impl App<Action = T> + 'static) -> Result<(), PlatformError> {
    let application = Application::new()?;
    let handler = ShellHandler::new(app);
    let mut builder = WindowBuilder::new(application.clone());
    builder.set_title("Druidinho");
    builder.set_size(Size::new(400., 400.));
    builder.set_handler(Box::new(handler));
    let window = builder.build()?;
    window.show();
    application.run(None);
    Ok(())
}
