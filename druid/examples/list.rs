// Copyright 2019 The Druid Authors.
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

//! Demos basic list widget and list manipulations.

use druid::im::{vector, Vector};
use druid::lens::{self, LensExt};
use druid::widget::{Button, CrossAxisAlignment, Either, Flex, Label, List, Scroll};
use druid::{
    AppLauncher, Color, Data, Lens, LocalizedString, UnitPoint, Widget, WidgetExt, WindowDesc,
};

#[derive(Clone, Data, Lens)]
struct AppData {
    left: Vector<u32>,
    right: Vector<usize>,
    // The selected list item on the right list, if any
    r_selected: Option<usize>,
    // Indexes for generating new items (we can't use the vector length because some items might
    // have been deleted).
    l_next: usize,
    r_next: usize,
}

impl AppData {
    /// Delete the item in the right list with the given id.
    ///
    /// Also unselect it if it is selected.
    fn del_right(&mut self, id: usize) {
        self.right.retain(|v| *v != id);
        if self.r_selected == Some(id) {
            self.r_selected = None;
        }
    }

    /// Toggle the selected list element.
    ///
    /// This shouldn't be called with an id that doesn't exist, but if it is nothing bad will
    /// happen.
    fn toggle_select(&mut self, id: usize) {
        if self.right.contains(&id) {
            self.r_selected = if self.is_selected(id) { None } else { Some(id) };
        }
    }

    /// whether the given list item id is currently selected.
    fn is_selected(&self, id: usize) -> bool {
        self.r_selected == Some(id)
    }
}

pub fn main() {
    let main_window = WindowDesc::new(ui_builder())
        .title(LocalizedString::new("list-demo-window-title").with_placeholder("List Demo"));
    // Set our initial data
    let left = vector![1, 2];
    let l_next = left.len() + 1;
    let right = vector![1, 2, 3];
    let r_next = right.len() + 1;
    let data = AppData {
        left,
        right,
        r_selected: None,
        l_next,
        r_next,
    };
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(data)
        .expect("launch failed");
}

fn ui_builder() -> impl Widget<AppData> {
    let mut root = Flex::column();

    // Build a button to add children to both lists
    root.add_child(
        Button::new("Add")
            .on_click(|_, data: &mut AppData, _| {
                // Add child to left list
                data.left.push_back(data.l_next as u32);
                data.l_next += 1;

                // Add child to right list
                data.right.push_back(data.r_next);
                data.r_next += 1;
            })
            .fix_height(30.0)
            .expand_width(),
    );

    let mut lists = Flex::row().cross_axis_alignment(CrossAxisAlignment::Start);

    // Build a simple list
    lists.add_flex_child(
        Scroll::new(List::new(|| {
            Label::new(|item: &u32, _env: &_| format!("List item #{}", item))
                .align_vertical(UnitPoint::LEFT)
                .padding(10.0)
                .expand()
                .height(50.0)
                .background(Color::rgb(0.5, 0.5, 0.5))
        }))
        .vertical()
        .lens(AppData::left),
        1.0,
    );

    // Build a list with shared data
    lists.add_flex_child(
        Scroll::new(List::new(|| {
            fn label(text_color: Color) -> impl Widget<(AppData, usize)> {
                Flex::row()
                    .with_child(
                        Label::new(|(_, item): &(AppData, usize), _env: &_| {
                            format!("List item #{}", item)
                        })
                        .with_text_color(text_color)
                        .align_vertical(UnitPoint::LEFT)
                        .on_click(
                            |_ctx, (shared, item): &mut (AppData, usize), _env| {
                                shared.toggle_select(*item);
                            },
                        ),
                    )
                    .with_flex_spacer(1.0)
                    .with_child(
                        Button::new("Delete")
                            .on_click(|_ctx, (shared, item): &mut (AppData, usize), _env| {
                                // We have access to both the list item's data and the app data.
                                // Remove element from right list.
                                shared.del_right(*item);
                            })
                            .fix_size(80.0, 20.0)
                            .align_vertical(UnitPoint::CENTER),
                    )
                    .padding(10.0)
                    .background(Color::rgb(0.5, 0.0, 0.5))
                    .fix_height(50.0)
            }
            Either::new(
                |(shared, id), _| shared.is_selected(*id),
                label(Color::WHITE),
                label(Color::BLACK),
            )
        }))
        .vertical()
        .lens(lens::Identity.map(
            // Expose shared data with children data
            |d: &AppData| (d.clone(), d.right.clone()),
            |d: &mut AppData, (new_d, _): (AppData, Vector<usize>)| {
                // If shared data was changed reflect the changes in our AppData
                *d = new_d;
            },
        )),
        1.0,
    );

    root.add_flex_child(lists, 1.0);

    root.with_child(Label::new("horizontal list"))
        .with_child(
            Scroll::new(
                List::new(|| {
                    Label::new(|item: &u32, _env: &_| format!("List item #{}", item))
                        .padding(10.0)
                        .background(Color::rgb(0.5, 0.5, 0.0))
                        .fix_height(50.0)
                })
                .horizontal()
                .with_spacing(10.)
                .lens(AppData::left),
            )
            .horizontal(),
        )
        .debug_paint_layout()
}
