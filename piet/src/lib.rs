#[cfg(target_os = "windows")]
#[macro_use]
extern crate direct2d;

#[cfg(target_os = "windows")]
extern crate directwrite;

#[cfg(target_os = "windows")]
pub mod windows {
    pub mod math;
    pub mod render_target;
    pub mod brush;
    pub mod write;
}

#[cfg(target_os = "windows")]
pub use windows::math;
#[cfg(target_os = "windows")]
pub use windows::render_target;
#[cfg(target_os = "windows")]
pub use windows::brush;
#[cfg(target_os = "windows")]
pub use windows::write;
#[cfg(target_os = "windows")]
pub use windows::render_target::RenderTarget;
#[cfg(target_os = "windows")]
pub use direct2d::Factory;
