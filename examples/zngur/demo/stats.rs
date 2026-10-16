// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! A Rust crate for C++ to use.

/// A running summary of values.
#[derive(Clone, Copy, Debug, Default)]
pub struct Summary {
    pub count: i32,
    pub total: i64,
}

impl Summary {
    pub fn add(&mut self, value: i32) {
        self.count += 1;
        self.total += i64::from(value);
    }

    /// `None` if there are no values. crubit skips it: `Option<f64>` is a generic type.
    pub fn mean(&self) -> Option<f64> {
        (self.count > 0).then(|| self.total as f64 / f64::from(self.count))
    }
}

/// A trait, which crubit skips.
pub trait Shape {
    fn area(&self) -> f64;
}

struct Square {
    side: f64,
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }
}

/// Returns a trait object, which crubit skips too.
pub fn make_square(side: f64) -> Box<dyn Shape> {
    Box::new(Square { side })
}
