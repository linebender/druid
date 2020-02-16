use super::clipboard::Clipboard;

pub struct Application;

impl Application {
    pub fn init() {
        // TODO: currently a no-op
    }

    pub fn quit() {
        unimplemented!(); // TODO
    }

    pub fn clipboard() -> Clipboard {
        unimplemented!(); // TODO
    }

    pub fn get_locale() -> String {
        // TODO
        "en-US".into()
    }
}