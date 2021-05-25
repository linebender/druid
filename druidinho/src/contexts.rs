use crate::kurbo::Rect;
use crate::piet::{Piet, PietText, RenderContext};
use druid_shell::WindowHandle;

use crate::widget_host::WidgetState;
use crate::widgets::layout::LayoutState;

pub struct EventCtx<'a> {
    pub(crate) window: &'a WindowHandle,
    pub(crate) state: &'a mut WidgetState,
    pub(crate) layout_state: &'a LayoutState,
}

pub struct PaintCtx<'a, 'b> {
    pub(crate) state: &'a WidgetState,
    pub(crate) layout_state: &'a LayoutState,
    pub(crate) render_ctx: &'a mut Piet<'b>,
}

pub struct LayoutCtx<'a> {
    pub(crate) window: &'a WindowHandle,
    pub(crate) state: &'a WidgetState,
    pub(crate) layout_state: &'a LayoutState,
}

impl<'a> EventCtx<'a> {
    pub fn text(&self) -> PietText {
        self.window.text()
    }

    pub fn hovered(&self) -> bool {
        self.layout_state.hovered
    }

    pub fn set_mouse_focus(&mut self, focus: bool) {
        self.state.mouse_focus = focus;
    }

    pub fn mouse_focused(&self) -> bool {
        self.state.mouse_focus
    }

    pub fn keyboard_focused(&self) -> bool {
        self.state.keyboard_focus
    }

    pub fn request_paint(&mut self) {
        self.window.invalidate();
    }

    pub fn request_update(&mut self) {
        self.state.request_update = true;
    }
}

impl LayoutCtx<'_> {
    pub fn text(&self) -> PietText {
        self.window.text()
    }
}

impl<'c> std::ops::Deref for PaintCtx<'_, 'c> {
    type Target = Piet<'c>;

    fn deref(&self) -> &Self::Target {
        self.render_ctx
    }
}

impl<'c> std::ops::DerefMut for PaintCtx<'_, 'c> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.render_ctx
    }
}

impl PaintCtx<'_, '_> {
    pub fn hovered(&self) -> bool {
        self.layout_state.hovered
    }

    pub fn mouse_focused(&self) -> bool {
        self.state.mouse_focus
    }

    pub fn keyboard_focused(&self) -> bool {
        self.state.keyboard_focus
    }

    pub fn frame(&self) -> Rect {
        self.layout_state.size.to_rect()
    }

    pub fn with_save(&mut self, f: impl FnOnce(&mut PaintCtx)) {
        if let Err(e) = self.render_ctx.save() {
            eprintln!("Failed to save RenderContext: '{}'", e);
            return;
        }

        f(self);

        if let Err(e) = self.render_ctx.restore() {
            eprintln!("Failed to restore RenderContext: '{}'", e);
        }
    }
}
