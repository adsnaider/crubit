// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Uses the types of another crate (`geo`), through its `.zng` file.

use geo::Point;

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub from: Point,
    pub to: Point,
}

impl Segment {
    pub fn new(from: Point, to: Point) -> Segment {
        Segment { from, to }
    }

    pub fn length2(&self) -> i32 {
        let dx = self.to.x - self.from.x;
        let dy = self.to.y - self.from.y;
        dx * dx + dy * dy
    }
}
