// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! A Rust crate for C++ to use.

use point::Point;

/// Uses `point::Point`, from another crate.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn new(min: Point, max: Point) -> Rect {
        Rect { min, max }
    }

    pub fn area(&self) -> i32 {
        (self.max.x - self.min.x) * (self.max.y - self.min.y)
    }

    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.min.x += dx;
        self.min.y += dy;
        self.max.x += dx;
        self.max.y += dy;
    }

    /// `Option<i32>` isn't in crubit's `.zng` file yet: see `example_extra.zng`.
    pub fn aspect_ratio(&self) -> Option<i32> {
        (self.max.x - self.min.x).checked_div(self.max.y - self.min.y)
    }
}

/// Not `Copy`, and owns memory: C++ owns it by value and drops it.
#[derive(Debug, Default)]
pub struct Canvas {
    rects: Vec<Rect>,
}

impl Canvas {
    pub fn draw(&mut self, rect: Rect) {
        self.rects.push(rect);
    }

    pub fn total_area(&self) -> i32 {
        self.rects.iter().map(Rect::area).sum()
    }
}
