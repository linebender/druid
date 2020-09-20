use crate::{Widget, WidgetPod};

/// Meta information for Widget derive
pub struct CompositeMeta<T> {
    /// Built widget
    pub widget: Option<WidgetPod<T, Box<dyn Widget<T>>>>,
}

impl<T> Default for CompositeMeta<T> {
    fn default() -> Self {
        CompositeMeta { widget: None }
    }
}
