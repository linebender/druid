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

//! This is an example of how you would implement the game of life with druid.
//! This example doesnt showcase anything specific in druid.

use std::ops::{Index, IndexMut};
use std::time::{Duration, Instant};

use druid::widget::prelude::*;
use druid::widget::{Button, Flex, Label, Slider};
use druid::{
    AppLauncher, Color, Data, Lens, MouseButton, Point, Rect, TimerToken, WidgetExt, WindowDesc,
};
use std::sync::Arc;

const GRID_SIZE: usize = 41;
const POOL_SIZE: usize = GRID_SIZE * GRID_SIZE;

const BACKGROUND: Color = Color::grey8(23);
static COLOURS: ColorScheme = &[
    Color::rgb8(0xEB, 0xF1, 0xF7), //Color::rgb(235, 241, 247)
    Color::rgb8(0xA3, 0xFC, 0xF7), //Color::rgb(162,252,247)
    Color::rgb8(0xA2, 0xE3, 0xD8), //Color::rgb(162,227,216)
    Color::rgb8(0xF2, 0xE6, 0xF1), //Color::rgb(242,230,241)
    Color::rgb8(0xE0, 0xAF, 0xAF), //Color::rgb(224,175,175)
];

#[allow(clippy::clippy::rc_buffer)]
#[derive(Clone, Data, PartialEq)]
struct Grid {
    storage: Arc<Vec<bool>>,
}

impl Grid {
    pub fn new() -> Grid {
        Grid {
            storage: Arc::new(vec![false; POOL_SIZE]),
        }
    }
    pub fn evolve(&mut self) {
        let mut indices_to_mutate: Vec<GridPos> = vec![];
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                let pos = GridPos { row, col };
                let n_lives_around = self.n_neighbors(pos);
                match (self[pos], n_lives_around) {
                    // death by overcrowding
                    (true, x) if x > 3 => indices_to_mutate.push(pos),
                    // death by loneliness
                    (true, x) if x < 2 => indices_to_mutate.push(pos),
                    // resurrection by life support
                    (false, 3) => indices_to_mutate.push(pos),
                    _ => (),
                };
            }
        }
        for pos_mut in indices_to_mutate {
            self[pos_mut] = !self[pos_mut];
        }
    }

    pub fn neighbors(pos: GridPos) -> [Option<GridPos>; 8] {
        let above = pos.above();
        let below = pos.below();
        let left = pos.left();
        let right = pos.right();
        let above_left = above.and_then(|pos| pos.left());
        let above_right = above.and_then(|pos| pos.right());
        let below_left = below.and_then(|pos| pos.left());
        let below_right = below.and_then(|pos| pos.right());
        [
            above,
            below,
            left,
            right,
            above_left,
            above_right,
            below_left,
            below_right,
        ]
    }

    pub fn n_neighbors(&self, pos: GridPos) -> usize {
        Grid::neighbors(pos)
            .iter()
            .filter(|x| x.is_some() && self[x.unwrap()])
            .count()
    }

    pub fn set_alive(&mut self, positions: &[GridPos]) {
        for pos in positions {
            self[*pos] = true;
        }
    }

    #[allow(dead_code)]
    pub fn set_dead(&mut self, positions: &[GridPos]) {
        for pos in positions {
            self[*pos] = false;
        }
    }

    pub fn clear(&mut self) {
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                self[GridPos { row, col }] = false;
            }
        }
    }
}

#[derive(Clone, Copy)]
struct GridPos {
    row: usize,
    col: usize,
}

impl GridPos {
    pub fn above(self) -> Option<GridPos> {
        if self.row == 0 {
            None
        } else {
            Some(GridPos {
                row: self.row - 1,
                col: self.col,
            })
        }
    }
    pub fn below(self) -> Option<GridPos> {
        if self.row == GRID_SIZE - 1 {
            None
        } else {
            Some(GridPos {
                row: self.row + 1,
                col: self.col,
            })
        }
    }
    pub fn left(self) -> Option<GridPos> {
        if self.col == 0 {
            None
        } else {
            Some(GridPos {
                row: self.row,
                col: self.col - 1,
            })
        }
    }
    pub fn right(self) -> Option<GridPos> {
        if self.col == GRID_SIZE - 1 {
            None
        } else {
            Some(GridPos {
                row: self.row,
                col: self.col + 1,
            })
        }
    }
}

type ColorScheme = &'static [Color];

#[derive(Clone, Lens, Data)]
struct AppData {
    grid: Grid,
    drawing: bool,
    paused: bool,
    updates_per_second: f64,
}

impl AppData {
    // allows time interval in the range [100ms, 5000ms]
    // equivalently, 0.2 ~ 20ups
    pub fn iter_interval(&self) -> u64 {
        (1000. / self.updates_per_second) as u64
    }
}

struct GameOfLifeWidget {
    timer_id: TimerToken,
    cell_size: Size,
    last_update: Instant,
}

impl GameOfLifeWidget {
    fn grid_pos(&self, p: Point) -> Option<GridPos> {
        let w0 = self.cell_size.width;
        let h0 = self.cell_size.height;
        if p.x < 0.0 || p.y < 0.0 || w0 == 0.0 || h0 == 0.0 {
            return None;
        }
        let row = (p.x / w0) as usize;
        let col = (p.y / h0) as usize;
        if row >= GRID_SIZE || col >= GRID_SIZE {
            return None;
        }
        Some(GridPos { row, col })
    }
}

impl Widget<AppData> for GameOfLifeWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppData, _env: &Env) {
        match event {
            Event::WindowConnected => {
                ctx.request_paint();
                let deadline = Duration::from_millis(data.iter_interval());
                self.last_update = Instant::now();
                self.timer_id = ctx.request_timer(deadline);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    if !data.paused {
                        data.grid.evolve();
                        ctx.request_paint();
                    }
                    let deadline = Duration::from_millis(data.iter_interval());
                    self.last_update = Instant::now();
                    self.timer_id = ctx.request_timer(deadline);
                }
            }
            Event::MouseDown(e) => {
                if e.button == MouseButton::Left {
                    data.drawing = true;
                    let grid_pos_opt = self.grid_pos(e.pos);
                    grid_pos_opt
                        .iter()
                        .for_each(|pos| data.grid[*pos] = !data.grid[*pos]);
                }
            }
            Event::MouseUp(e) => {
                if e.button == MouseButton::Left {
                    data.drawing = false;
                }
            }
            Event::MouseMove(e) => {
                if data.drawing {
                    if let Some(grid_pos_opt) = self.grid_pos(e.pos) {
                        data.grid[grid_pos_opt] = true
                    }
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppData,
        _env: &Env,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppData, data: &AppData, _env: &Env) {
        if (data.updates_per_second - old_data.updates_per_second).abs() > 0.001 {
            let deadline = Duration::from_millis(data.iter_interval())
                .checked_sub(Instant::now().duration_since(self.last_update))
                .unwrap_or_else(|| Duration::from_secs(0));
            self.timer_id = ctx.request_timer(deadline);
        }
        if data.grid != old_data.grid {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppData,
        _env: &Env,
    ) -> Size {
        let max_size = bc.max();
        let min_side = max_size.height.min(max_size.width);
        Size {
            width: min_side,
            height: min_side,
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, _env: &Env) {
        let size: Size = ctx.size();
        let w0 = size.width / GRID_SIZE as f64;
        let h0 = size.height / GRID_SIZE as f64;
        let cell_size = Size {
            width: w0,
            height: h0,
        };
        self.cell_size = cell_size;
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                let pos = GridPos { row, col };
                if data.grid[pos] {
                    let point = Point {
                        x: w0 * row as f64,
                        y: h0 * col as f64,
                    };
                    let rect = Rect::from_origin_size(point, cell_size);

                    // We devide by 2 so that the colour changes every 2 positions instead of every 1
                    ctx.fill(
                        rect,
                        &COLOURS[((pos.row * GRID_SIZE + pos.col) / 2) % COLOURS.len()],
                    );
                }
            }
        }
    }
}

// gives back positions of a glider pattern
// _____
// |  *|
// |* *|
// | **|
// ‾‾‾‾‾
fn glider(left_most: GridPos) -> Option<[GridPos; 5]> {
    let center = left_most.right()?;
    Some([
        left_most,
        center.below()?.right()?,
        center.below()?,
        center.right()?,
        center.above()?.right()?,
    ])
}

// gives back positions of a blinker pattern
// ___
// |*|
// |*|
// |*|
// ‾‾‾
fn blinker(top: GridPos) -> Option<[GridPos; 3]> {
    let center = top.below()?;
    Some([top, center, center.below()?])
}

fn make_widget() -> impl Widget<AppData> {
    Flex::column()
        .with_flex_child(
            GameOfLifeWidget {
                timer_id: TimerToken::INVALID,
                cell_size: Size {
                    width: 0.0,
                    height: 0.0,
                },
                last_update: Instant::now(),
            },
            1.0,
        )
        .with_child(
            Flex::column()
                .with_child(
                    // a row with two buttons
                    Flex::row()
                        .with_flex_child(
                            // pause / resume button
                            Button::new(|data: &bool, _: &Env| match data {
                                true => "Resume".into(),
                                false => "Pause".into(),
                            })
                            .on_click(|ctx, data: &mut bool, _: &Env| {
                                *data = !*data;
                                ctx.request_layout();
                            })
                            .lens(AppData::paused)
                            .padding((5., 5.)),
                            1.0,
                        )
                        .with_flex_child(
                            // clear button
                            Button::new("Clear")
                                .on_click(|ctx, data: &mut Grid, _: &Env| {
                                    data.clear();
                                    ctx.request_paint();
                                })
                                .lens(AppData::grid)
                                .padding((5., 5.)),
                            1.0,
                        )
                        .padding(8.0),
                )
                .with_child(
                    Flex::row()
                        .with_child(
                            Label::new(|data: &AppData, _env: &_| {
                                format!("{:.2}updates/s", data.updates_per_second)
                            })
                            .padding(3.0),
                        )
                        .with_flex_child(
                            Slider::new()
                                .with_range(0.2, 20.0)
                                .expand_width()
                                .lens(AppData::updates_per_second),
                            1.,
                        )
                        .padding(8.0),
                )
                .background(BACKGROUND),
        )
}

pub fn main() {
    let window = WindowDesc::new(make_widget())
        .window_size(Size {
            width: 800.0,
            height: 800.0,
        })
        .resizable(false)
        .title("Game of Life");
    let mut grid = Grid::new();
    let pattern0 = glider(GridPos { row: 5, col: 5 });
    if let Some(x) = pattern0 {
        grid.set_alive(&x);
    }
    let pattern1 = blinker(GridPos { row: 29, col: 29 });
    if let Some(x) = pattern1 {
        grid.set_alive(&x);
    }

    AppLauncher::with_window(window)
        .log_to_console()
        .launch(AppData {
            grid,
            drawing: false,
            paused: false,
            updates_per_second: 10.0,
        })
        .expect("launch failed");
}

impl Index<GridPos> for Grid {
    type Output = bool;
    fn index(&self, pos: GridPos) -> &Self::Output {
        let idx = pos.row * GRID_SIZE + pos.col;
        &self.storage[idx]
    }
}

impl IndexMut<GridPos> for Grid {
    fn index_mut(&mut self, pos: GridPos) -> &mut Self::Output {
        let idx = pos.row * GRID_SIZE + pos.col;
        Arc::make_mut(&mut self.storage).index_mut(idx)
    }
}
