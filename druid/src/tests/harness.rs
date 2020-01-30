// Copyright 2020 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tools and infrastructure for testing widgets.

use crate::core::{BaseState, CommandQueue};
use crate::piet::{Device, Piet};
use crate::*;

/// Mother, tell me: where do contexts come from?
struct Mocks<'a> {
    ctx: MockWinCtx<'a>,
    handle: WindowHandle,
    command_queue: CommandQueue,
    cursor: Option<Cursor>,
    base_state: BaseState,
    window_id: WindowId,
}

/// A `WinCtx` impl that we can conjure from the ether.
pub struct MockWinCtx<'a> {
    piet: Piet<'a>,
}

impl<'a> WinCtx<'a> for MockWinCtx<'a> {
    fn invalidate(&mut self) {}
    fn text_factory(&mut self) -> &mut Text {
        self.piet.text()
    }

    fn set_cursor(&mut self, cursor: &Cursor) {}
    //TODO: we could actually implement timers if we were ambitious
    fn request_timer(&mut self, _deadline: std::time::Instant) -> TimerToken {
        TimerToken::next()
    }
    fn open_file_sync(&mut self, _: FileDialogOptions) -> Option<FileInfo> {
        None
    }
    fn save_as_sync(&mut self, _: FileDialogOptions) -> Option<FileInfo> {
        None
    }
}

impl<'a> MockWinCtx<'a> {
    fn new(piet: Piet<'a>) -> Self {
        MockWinCtx { piet }
    }
}

impl<'a> Mocks<'a> {
    fn with_mocks(mut f: impl FnMut(&mut Mocks)) {
        let mut device = Device::new().expect("harness failed to get device");
        let mut target = device
            .bitmap_target(400, 400, 100.)
            .expect("harness failed to create target");
        let ctx = MockWinCtx::new(target.render_context());
        let mut mocks = Mocks {
            ctx,
            handle: Default::default(),
            command_queue: Default::default(),
            cursor: Default::default(),
            base_state: BaseState::new(WidgetId::next()),
            //FIXME: makee these const or someethign idk
            window_id: WindowId::next(),
        };

        f(&mut mocks);
    }

    fn event_ctx<'me>(&'me mut self) -> EventCtx<'me, 'a> {
        EventCtx {
            win_ctx: &mut self.ctx,
            cursor: &mut self.cursor,
            command_queue: &mut self.command_queue,
            window_id: self.window_id,
            window: &self.handle,
            base_state: &mut self.base_state,
            focus_widget: None,
            had_active: false,
            is_handled: false,
            is_root: true,
        }
    }

    fn lifecycle_ctx(&mut self) -> LifeCycleCtx {
        LifeCycleCtx {
            command_queue: &mut self.command_queue,
            children: Default::default(),
            window_id: self.window_id,
            widget_id: self.base_state.id,
            focus_widgets: Vec::new(),
            request_anim: false,
            needs_inval: false,
            children_changed: false,
        }
    }

    fn update_ctx(&mut self) -> UpdateCtx {
        UpdateCtx {
            text_factory: self.ctx.text_factory(),
            window: &self.handle,
            needs_inval: false,
            children_changed: false,
            window_id: self.window_id,
            widget_id: self.base_state.id,
        }
    }

    fn layout_ctx(&mut self) -> LayoutCtx {
        LayoutCtx {
            text_factory: self.ctx.text_factory(),
            window_id: self.window_id,
        }
    }

    fn paint_ctx<'me>(&'me mut self) -> PaintCtx<'me, 'a> {
        PaintCtx {
            render_ctx: &mut self.ctx.piet,
            window_id: self.window_id,
            region: Rect::ZERO.into(),
            base_state: &self.base_state,
            focus_widget: None,
        }
    }
}
