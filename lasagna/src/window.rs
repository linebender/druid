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

//! A container for a window.

use druid_shell::kurbo::{Line, Point};
use druid_shell::piet::{Color, RenderContext};

use crate::element::{Action, Button, Element};
use crate::tree::Mutation;

pub struct Window {
    app_logic: Box<dyn FnMut(Vec<Action>) -> Mutation>,
    button: Button,
}

const FG_COLOR: Color = Color::rgb8(0xf0, 0xf0, 0xea);

impl Window {
    pub fn new(app_logic: Box<dyn FnMut(Vec<Action>) -> Mutation>) -> Window {
        Window {
            app_logic,
            button: Default::default(),
        }
    }

    pub fn paint(&mut self, piet: &mut druid_shell::piet::Piet) {
        piet.stroke(Line::new((10.0, 50.0), (90.0, 90.0)), &FG_COLOR, 1.0);
        self.button.layout();
        self.button.paint(piet, Point::new(0.0, 0.0));
    }
}
