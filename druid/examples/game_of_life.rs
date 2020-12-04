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

#![allow(clippy::unreadable_literal)]

//! Game of life

use std::ops::{Index, IndexMut};
use std::time::{Duration, Instant};

use druid::widget::prelude::*;
use druid::widget::{Button, Flex, Label, Slider};
use druid::{
    AppLauncher, Color, Data, Lens, LocalizedString, MouseButton, Point, Rect, TimerToken,
    WidgetExt, WindowDesc,
};
use std::sync::Arc;

const GRID_SIZE: usize = 40;
const POOL_SIZE: usize = GRID_SIZE * GRID_SIZE;

const BG: Color = Color::grey8(23 as u8);
const C0: Color = Color::from_rgba32_u32(0xEBF1F7);
const C1: Color = Color::from_rgba32_u32(0xA3FCF7);
const C2: Color = Color::from_rgba32_u32(0xA2E3D8);
const C3: Color = Color::from_rgba32_u32(0xF2E6F1);
const C4: Color = Color::from_rgba32_u32(0xE0AFAF);

#[allow(clippy::clippy::rc_buffer)]
#[derive(Clone, Data)]
struct Grid {
    storage: Arc<Vec<bool>>,
}

#[derive(Clone, Copy, PartialEq)]
struct GridPos {
    row: usize,
    col: usize,
}

#[derive(Clone)]
struct ColorScheme {
    colors: Vec<Color>,
    current: usize,
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
    #[allow(dead_code)]
    pub fn above_left(self) -> Option<GridPos> {
        self.above().and_then(|pos| pos.left())
    }
    pub fn above_right(self) -> Option<GridPos> {
        self.above().and_then(|pos| pos.right())
    }
    #[allow(dead_code)]
    pub fn below_left(self) -> Option<GridPos> {
        self.below().and_then(|pos| pos.left())
    }
    pub fn below_right(self) -> Option<GridPos> {
        self.below().and_then(|pos| pos.right())
    }
}

#[derive(Clone, Lens, Data)]
struct AppData {
    grid: Grid,
    drawing: bool,
    paused: bool,
    speed: f64,
}

impl AppData {
    // allows time interval in the range [100ms, 5000ms]
    // equivalently, 0.2 ~ 20fps
    pub fn iter_interval(&self) -> u64 {
        (1000. / self.fps()) as u64
    }
    pub fn fps(&self) -> f64 {
        self.speed.max(0.01) * 20.0
    }
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
                let life = self[pos];
                // death by loneliness or overcrowding
                if life && (n_lives_around < 2 || n_lives_around > 3) {
                    indices_to_mutate.push(pos);
                    continue;
                }
                // resurrection by life support
                if !life && n_lives_around == 3 {
                    indices_to_mutate.push(pos);
                }
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

struct GameOfLifeWidget {
    timer_id: TimerToken,
    cell_size: Size,
    color_scheme: ColorScheme,
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
        if (data.fps() - old_data.fps()).abs() > 0.001 {
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
                    ctx.fill(rect, &self.color_scheme[pos]);
                }
            }
        }
    }
}

// gives back positions of a glider pattern
//   *
// * *
//  **
fn glider(left_most: GridPos) -> Option<[GridPos; 5]> {
    if left_most.row < 1 || left_most.row > GRID_SIZE - 2 || left_most.col > GRID_SIZE - 3 {
        return None;
    }
    let center = left_most.right().unwrap();
    Some([
        left_most,
        center.below_right().unwrap(),
        center.below().unwrap(),
        center.right().unwrap(),
        center.above_right().unwrap(),
    ])
}

// gives back positions of a blinker pattern
//  *
//  *
//  *
fn blinker(top: GridPos) -> Option<[GridPos; 3]> {
    if top.row > GRID_SIZE - 3 || top.col < 1 || top.col > GRID_SIZE - 2 {
        return None;
    }
    let center = top.below().unwrap();
    Some([top, center, center.below().unwrap()])
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
                color_scheme: Default::default(),
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
                            Label::new(|data: &AppData, _env: &_| format!("{:.2}FPS", data.fps()))
                                .padding(3.0),
                        )
                        .with_flex_child(Slider::new().expand_width().lens(AppData::speed), 1.)
                        .padding(8.0),
                )
                .background(BG),
        )
}

pub fn main() {
    let window = WindowDesc::new(make_widget)
        .window_size(Size {
            width: 800.0,
            height: 800.0,
        })
        .resizable(false)
        .title(
            LocalizedString::new("custom-widget-demo-window-title")
                .with_placeholder("Game of Life"),
        );
    let mut grid = Grid::new();
    let pattern0 = glider(GridPos { row: 5, col: 5 });
    for x in &pattern0 {
        grid.set_alive(x);
    }
    let pattern1 = blinker(GridPos { row: 29, col: 29 });
    for x in &pattern1 {
        grid.set_alive(x);
    }
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(AppData {
            grid,
            drawing: false,
            paused: false,
            speed: 0.5,
        })
        .expect("launch failed");
}

impl Index<GridPos> for ColorScheme {
    type Output = Color;
    fn index(&self, pos: GridPos) -> &Self::Output {
        let idx = pos.row * GRID_SIZE + pos.col;
        self.colors.index(idx % self.colors.len())
    }
}

impl Index<GridPos> for Grid {
    type Output = bool;
    fn index(&self, pos: GridPos) -> &Self::Output {
        let idx = pos.row * GRID_SIZE + pos.col;
        self.storage.index(idx)
    }
}

impl IndexMut<GridPos> for Grid {
    fn index_mut(&mut self, pos: GridPos) -> &mut Self::Output {
        let idx = pos.row * GRID_SIZE + pos.col;
        Arc::make_mut(&mut self.storage).index_mut(idx)
    }
}

impl PartialEq for Grid {
    fn eq(&self, other: &Self) -> bool {
        for i in 0..POOL_SIZE {
            if self.storage[i as usize] != other.storage[i as usize] {
                return false;
            }
        }
        true
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        ColorScheme {
            colors: vec![C0, C1, C2, C3, C4],
            current: 0,
        }
    }
}
