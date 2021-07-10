// Copyright 2021 The Druid Authors.
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

use std::any::Any;

use druid_shell::kurbo::{Point, Size, Vec2};
use druid_shell::piet::{Color, Piet, RenderContext, Text, TextLayoutBuilder};

use crate::tree::{Id, Mutation, MutationEl};

/// The trait for (widget-like) render elements.
///
/// We can envision other elements (like menus and windows), but
/// keep things simple for now.
pub trait Element {
    fn mutate(&mut self, mutation: Mutation);

    fn event(&mut self, event: &Event, id: Id, actions: &mut Vec<Action>);

    fn layout(&mut self) -> Size;

    fn paint(&mut self, ctx: &mut Piet, pos: Point);
}

pub struct Pod {
    id: Id,
    size: Size,
    element: Box<dyn Element>,
}

pub struct Action {
    pub id: Id,
    pub action: Box<dyn Any>,
}

#[derive(Debug)]
pub enum Event {
    MouseDown(Point),
}

impl Pod {
    pub fn new(id: Id, element: Box<dyn Element>) -> Pod {
        Pod {
            id,
            size: Default::default(),
            element,
        }
    }

    pub fn mutate(&mut self, mutation: Mutation) {
        self.element.mutate(mutation)
    }
}

#[derive(Default)]
pub struct Button(String);

pub enum ButtonCmd {
    SetText(String),
}

impl Element for Button {
    fn mutate(&mut self, mutation: Mutation) {
        if let Some(cmd) = mutation.cmds {
            if let Ok(cmd) = cmd.downcast() {
                match *cmd {
                    ButtonCmd::SetText(s) => self.0 = s,
                }
            }
        }
    }

    fn event(&mut self, _event: &Event, id: Id, actions: &mut Vec<Action>) {
        actions.push(Action {
            id,
            action: Box::new(()),
        })
    }

    fn layout(&mut self) -> Size {
        Size::new(100., 20.)
    }

    fn paint(&mut self, ctx: &mut Piet, pos: Point) {
        let layout = ctx
            .text()
            .new_text_layout(self.0.clone())
            .text_color(Color::WHITE)
            .build()
            .unwrap();
        ctx.draw_text(&layout, pos);
    }
}

#[derive(Default)]
pub struct Column {
    children: Vec<Pod>,
}

impl Element for Column {
    fn mutate(&mut self, mutation: Mutation) {
        let mut i = 0;
        for op in mutation.child {
            match op {
                MutationEl::Skip(n) => i += n,
                MutationEl::Insert(id, child, child_mut) => {
                    if let Ok(el) = child.downcast() {
                        let mut pod = Pod::new(id, *el);
                        pod.mutate(child_mut);
                        self.children.insert(i, pod);
                        i += 1;
                    }
                }
                MutationEl::Update(child_mut) => {
                    self.children[i].mutate(child_mut);
                    i += 1;
                }
                MutationEl::Delete(n) => {
                    self.children.drain(i..i + n);
                }
            }
        }
    }

    fn event(&mut self, event: &Event, _id: Id, actions: &mut Vec<Action>) {
        match event {
            Event::MouseDown(p) => {
                let mut p = *p;
                for child in &mut self.children {
                    if p.y < child.size.height {
                        let child_event = Event::MouseDown(p);
                        child.element.event(&child_event, child.id, actions);
                        break;
                    } else {
                        p.y -= child.size.height;
                    }
                }
            }
        }
    }

    fn layout(&mut self) -> Size {
        let mut size = Size::default();
        for child in &mut self.children {
            let child_size = child.element.layout();
            child.size = child_size;
            size.width = child_size.width.max(child_size.width);
            size.height += child_size.height;
        }
        size
    }

    fn paint(&mut self, ctx: &mut Piet, pos: Point) {
        let mut child_pos = pos + Vec2::new(10.0, 0.0);
        for child in &mut self.children {
            child.element.paint(ctx, child_pos);
            child_pos.y += child.size.height;
        }
    }
}
