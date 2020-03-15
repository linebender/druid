// Copyright 2019 The xi-editor Authors.
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

//! A clickable widget.

use crate::widget::Controller;
use crate::{Data, Env, Event, EventCtx, LifeCycle, LifeCycleCtx, UpdateCtx, Widget};

pub struct Click<T> {
    action: Box<dyn Fn(&mut T, &mut Env)>,
}

impl<T: Data> Click<T> {
    pub fn new(action: impl Fn(&mut T, &mut Env) + 'static) -> Self {
        Click {
            action: Box::new(action),
        }
    }
}

impl<T: Data, W: Widget<T>> Controller<T, W> for Click<T> {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        let mut new_env = env.clone();
                        (self.action)(data, &mut new_env);
                        ctx.request_paint();
                        child.event(ctx, event, data, &new_env);
                    }
                }
            }
            _ => (),
        }
    }

    // fn lifecycle(
    //     &mut self,
    //     child: &mut W,
    //     ctx: &mut LifeCycleCtx,
    //     event: &LifeCycle,
    //     data: &T,
    //     env: &Env,
    // ) {
    //     let mut new_env = env.clone();
    //     let mut new_data = data.clone();
    //     (self.action)(&mut new_data, &mut new_env);
    //     child.lifecycle(ctx, event, &mut new_data, env)
    // }

    // fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
    //     let mut new_env = env.clone();
    //     let mut new_data = data.clone();
    //     (self.action)(&mut new_data, &mut new_env);
    //     child.update(ctx, old_data, &new_data, &new_env)
    // }
}
