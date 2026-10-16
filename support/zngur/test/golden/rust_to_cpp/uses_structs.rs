// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Types of other crates come from their `.zng` files.

use structs::Point;

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub from: Point,
    pub to: Point,
}

impl Line {
    pub fn new(from: Point, to: Point) -> Line {
        Line { from, to }
    }

    pub fn length2(&self) -> i32 {
        let dx = self.to.x - self.from.x;
        let dy = self.to.y - self.from.y;
        dx * dx + dy * dy
    }
}

pub fn origin() -> Point {
    Point::default()
}

pub fn x_of(point: &Point) -> i32 {
    point.x
}
