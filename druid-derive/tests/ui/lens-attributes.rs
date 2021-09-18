#![allow(dead_code)]

use druid::*;

#[derive(Lens)]
struct Item {
    #[lens(name = "count_lens")]
    count: usize,
    #[lens(ignore)]
    complete: bool,
}

impl Item {
    fn count(&self) -> usize {
        self.count
    }
    fn complete(&mut self) {
        self.complete = true;
    }
}

fn main() {}
