use win_main;

pub struct Application {}

impl Application {
    pub fn quit() {
        win_main::request_quit();
    }
}
