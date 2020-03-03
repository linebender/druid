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

use druid::kurbo::BezPath;
use druid::piet::{FontBuilder, ImageFormat, InterpolationMode, Text, TextLayoutBuilder};


use druid::{Affine, AppLauncher, BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Widget, WindowDesc, Data};


const POOL_SIZE: i32 = 1600;
const CELL_COLOR: Color = Color::rgb8(0xf3 as u8, 0xf4 as u8, 8 as u8);

#[derive(Clone)]
struct AppData {
    grid: [i32; 1600],
}

impl AppData {
    pub fn evolve(&mut self) {
        let mut indices_to_mutate: Vec<usize> = vec![];
        for i in 0..POOL_SIZE {
            let neighbors = AppData::neighbors(i);
            let mut sum = 0;
            for nb_i in &neighbors {
                if *nb_i >= 0 && *nb_i < POOL_SIZE {
                    let idx = *nb_i as usize;
                    let neighbor_life = self.grid[idx];
                    if neighbor_life > 0 {
                        sum += neighbor_life;
                    }
                }
            }
            let life = self.grid[i as usize];
            // death by loneliness or overcrowding
            if life == 1 && (sum < 2 || sum > 3) {
                indices_to_mutate.push(i as usize);
                continue;
            }
            if life == -1 && sum == 3 {
                indices_to_mutate.push(i as usize);
            }
        }
        println!("mut idx: {:?}", indices_to_mutate);
        for i_mut in indices_to_mutate {
            self.grid[i_mut] *= -1;
        }
    }

    // return the indices of neighbors of a cell, clockwise, starting from top-left corner
    pub fn neighbors(i: i32) -> [i32; 8]{
        return [i-41, i-40, i-39, i+1, i+41, i+40, i+39, i-1];
    }
}

impl Data for AppData {
    fn same(&self, other: &Self) -> bool {
        for i in 0..POOL_SIZE {
            if self.grid[i as usize] != other.grid[i as usize] {
                return false;
            }
        }
        return true;
    }
}

struct CustomWidget;


impl Widget<AppData> for CustomWidget {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut AppData, _env: &Env) {
        match _event {
            Event::MouseDown(_e) => {
                _data.evolve();
                _ctx.request_paint();
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
    ) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &AppData, _data: &AppData, _env: &Env) {}

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
        let w0 = size.width/40.0;
        let h0 = size.height/40.0;
        let cell_size = Size{ width: w0, height: h0 };
        for i in 0..1600 {
            if data.grid[i] == 1 {
                let xi = i % 40;
                let yi =  i / 40;
                let pos = Point{ x: w0 * xi as f64, y: h0 * yi as f64};
                let rect = Rect::from_origin_size(pos, cell_size);
                paint_ctx.fill(rect, &CELL_COLOR);
            }
        }
    }
}

fn main() {
    let window = WindowDesc::new(|| CustomWidget {}).title(
        LocalizedString::new("custom-widget-demo-window-title").with_placeholder("Game of Life"),
    );
    let mut grid = [-1; 1600];
    // glider
    grid[40] = 1;
    grid[81] = 1;
    grid[82] = 1;
    grid[42] = 1;
    grid[2] = 1;
    // blinker
    grid[54] = 1;
    grid[55] = 1;
    grid[56] = 1;
    // toad
    grid[375] = 1;
    grid[376] = 1;
    grid[377] = 1;
    grid[416] = 1;
    grid[417] = 1;
    grid[418] = 1;
    // penta-decathlon
    grid[72] = 1;
    grid[112] = 1;
    grid[192] = 1;
    grid[232] = 1;
    grid[272] = 1;
    grid[312] = 1;
    grid[392] = 1;
    grid[432] = 1;
    grid[151] = 1;
    grid[153] = 1;
    grid[351] = 1;
    grid[353] = 1;
    AppLauncher::with_window(window)
        .use_simple_logger()
        .launch(AppData{ grid })
        .expect("launch failed");
}

