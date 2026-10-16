// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Structs: layout, fields, well-known traits and methods.

/// `Copy`: C++ copies it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

    pub fn into_sum(self) -> i64 {
        i64::from(self.x) + i64::from(self.y)
    }

    /// Clashes with the field `x`, so it's renamed.
    pub fn x(&self) -> i32 {
        self.x
    }
}

/// `Clone` without `Copy`, and private fields.
#[derive(Clone, Debug, Default)]
pub struct Named {
    pub id: u64,
    name: String,
}

impl Named {
    pub fn name_len(&self) -> usize {
        self.name.len()
    }
}

/// A destructor: C++ drops it.
pub struct Guard {
    value: i32,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.value = 0;
    }
}

impl Guard {
    pub fn new(value: i32) -> Guard {
        Guard { value }
    }

    pub fn value(&self) -> i32 {
        self.value
    }
}

/// Tuple structs and unit structs.
#[derive(Clone, Copy)]
pub struct Pair(pub i32, pub f64);

#[derive(Clone, Copy, Default)]
pub struct Empty;
