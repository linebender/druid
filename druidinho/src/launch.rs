use crate::kurbo::Size;

use druid_shell::{Application, Error as PlatformError, WindowBuilder};

use crate::shell_handler::ShellHandler;
use crate::Widget;

pub fn launch(widget: impl Widget + 'static) -> Result<(), PlatformError> {
    let app = Application::new()?;
    let handler = ShellHandler::new(widget);
    let mut builder = WindowBuilder::new(app.clone());
    builder.set_title("Druidinho");
    builder.set_size(Size::new(400., 400.));
    builder.set_handler(Box::new(handler));
    let window = builder.build()?;
    window.show();
    app.run(None);
    Ok(())
}
