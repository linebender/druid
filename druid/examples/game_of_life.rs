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

//! Game of life

use std::ops::{Index, IndexMut};
use std::time::{Duration, Instant};

use druid::{
    AppLauncher, BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle,
    LifeCycleCtx, LocalizedString, MouseButton, PaintCtx, Point, Rect, RenderContext, Size,
    TimerToken, UpdateCtx, Widget, WindowDesc,
};
use std::sync::Arc;

const GRID_SIZE: usize = 40;
const POOL_SIZE: usize = GRID_SIZE * GRID_SIZE;
const CELL_COLOR: Color = Color::rgb8(0xf3 as u8, 0xf4 as u8, 8 as u8);

#[derive(Clone)]
struct Grid {
    storage: Arc<Vec<bool>>,
}

#[derive(Clone, Copy, PartialEq)]
struct GridPos {
    row: usize,
    col: usize,
}

impl GridPos {
    pub fn above(&self) -> Option<GridPos> {
        if self.row == 0 {
            None
        } else {
            Some(GridPos {
                row: self.row - 1,
                col: self.col,
            })
        }
    }
    pub fn below(&self) -> Option<GridPos> {
        if self.row == GRID_SIZE - 1 {
            None
        } else {
            Some(GridPos {
                row: self.row + 1,
                col: self.col,
            })
        }
    }
    pub fn left(&self) -> Option<GridPos> {
        if self.col == 0 {
            None
        } else {
            Some(GridPos {
                row: self.row,
                col: self.col - 1,
            })
        }
    }
    pub fn right(&self) -> Option<GridPos> {
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
    pub fn above_left(&self) -> Option<GridPos> {
        self.above().and_then(|pos| pos.left())
    }
    pub fn above_right(&self) -> Option<GridPos> {
        self.above().and_then(|pos| pos.right())
    }
    #[allow(dead_code)]
    pub fn below_left(&self) -> Option<GridPos> {
        self.below().and_then(|pos| pos.left())
    }
    pub fn below_right(&self) -> Option<GridPos> {
        self.below().and_then(|pos| pos.right())
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
        return true;
    }
}

#[derive(Clone)]
struct AppData {
    grid: Grid,
    drawing: bool,
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
}

impl Data for AppData {
    fn same(&self, other: &Self) -> bool {
        self.grid == other.grid
    }
}

struct GameOfLifeWidget {
    timer_id: TimerToken,
    cell_size: Size,
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
                let deadline = Instant::now() + Duration::from_millis(550);
                self.timer_id = ctx.request_timer(deadline);
            }
            Event::Timer(id) => {
                if *id == self.timer_id {
                    data.grid.evolve();
                    ctx.request_paint();
                    let deadline = Instant::now() + Duration::from_millis(550);
                    self.timer_id = ctx.request_timer(deadline);
                }
            }
            Event::MouseDown(e) => {
                if e.button == MouseButton::Left {
                    data.drawing = true;
                    let grid_pos_opt = self.grid_pos(e.pos);
                    grid_pos_opt.iter().for_each(|pos| data.grid[*pos] = true);
                }
            }
            Event::MouseUp(e) => {
                if e.button == MouseButton::Left {
                    data.drawing = false;
                }
            }
            Event::MouseMoved(e) => {
                if data.drawing {
                    let grid_pos_opt = self.grid_pos(e.pos);
                    grid_pos_opt.iter().for_each(|pos| data.grid[*pos] = true);
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

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &AppData, _data: &AppData, _env: &Env) {
        ctx.request_paint();
    }

    fn layout(
        &mut self,
        _layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &AppData,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, data: &AppData, _env: &Env) {
        let size: Size = paint_ctx.size();
        let w0 = size.width / 40.0;
        let h0 = size.height / 40.0;
        let cell_size = Size {
            width: w0,
            height: h0,
        };
        self.cell_size = cell_size;
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                let pos = GridPos { row, col };
                if data.grid[pos] {
                    let pos = Point {
                        x: w0 * row as f64,
                        y: h0 * col as f64,
                    };
                    let rect = Rect::from_origin_size(pos, cell_size);
                    paint_ctx.fill(rect, &CELL_COLOR);
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

fn main() {
    let window = WindowDesc::new(|| GameOfLifeWidget {
        timer_id: TimerToken::INVALID,
        cell_size: Size {
            width: 0.0,
            height: 0.0,
        },
    })
    .title(
        LocalizedString::new("custom-widget-demo-window-title").with_placeholder("Game of Life"),
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
        })
        .expect("launch failed");
}
