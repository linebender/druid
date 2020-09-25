use crate::app::{PendingWindow, WindowConfig};
use crate::lens::Unit;
use crate::win_handler::AppState;
use crate::{Data, Widget, WidgetExt, WidgetId, WindowHandle, WindowId};
use druid_shell::Error;

// We can't have any type arguments here, as both ends would need to know them
// ahead of time in order to instantiate correctly.
// So we erase everything to ()
/// The required information to create a sub window, including the widget it should host, and the
pub struct SubWindowRequirement {
    pub(crate) host_id: Option<WidgetId>, // Present if updates should be sent from the pod to the sub window.
    pub(crate) sub_window_root: Box<dyn Widget<()>>,
    pub(crate) window_config: WindowConfig,
    /// The window id that the sub window will have once it is created. Can be used to send commands to.
    pub window_id: WindowId,
}

impl SubWindowRequirement {
    pub(crate) fn new(
        host_id: Option<WidgetId>,
        sub_window_root: Box<dyn Widget<()>>,
        window_config: WindowConfig,
        window_id: WindowId,
    ) -> Self {
        SubWindowRequirement {
            host_id,
            sub_window_root,
            window_config,
            window_id,
        }
    }

    pub(crate) fn make_sub_window<T: Data>(
        self,
        app_state: &mut AppState<T>,
    ) -> Result<WindowHandle, Error> {
        let sub_window_root = self.sub_window_root;
        let pending = PendingWindow::new(|| sub_window_root.lens(Unit::default()));
        app_state.build_native_window(self.window_id, pending, self.window_config)
    }
}
