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

use druid_shell::kurbo::{Point, Size, Vec2};
use druid_shell::piet::{Color, Piet, RenderContext, Text, TextLayoutBuilder};

use crate::tree::{Id, Mutation, MutationEl};

/// The trait for (widget-like) render elements.
///
/// We can envision other elements (like menus and windows), but
/// keep things simple for now.
pub trait Element {
    fn mutate(&mut self, mutation: Mutation);

    fn layout(&mut self) -> Size;

    fn paint(&mut self, ctx: &mut Piet, pos: Point);
}

pub struct Pod {
    id: Id,
    size: Size,
    element: Box<dyn Element>,
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
pub struct Button;

impl Element for Button {
    fn mutate(&mut self, _mutation: Mutation) {}

    fn layout(&mut self) -> Size {
        Size::new(100., 20.)
    }

    fn paint(&mut self, ctx: &mut Piet, pos: Point) {
        let layout = ctx
            .text()
            .new_text_layout("text")
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
