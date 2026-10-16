// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! A Rust crate for C++ to use, through zngur.

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }

    pub fn norm2(&self) -> i32 {
        self.x * self.x + self.y * self.y
    }

    pub fn scale(&mut self, factor: i32) {
        self.x *= factor;
        self.y *= factor;
    }
}

/// Not `Copy`, and has a destructor: C++ owns it by value and drops it.
#[derive(Debug, Default)]
pub struct Path {
    points: Vec<Point>,
}

impl Path {
    pub fn push(&mut self, point: Point) {
        self.points.push(point);
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }
}

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn checked_div(a: i32, b: i32) -> Option<i32> {
    a.checked_div(b)
}
