// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Enums are opaque values.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Color {
    #[default]
    Red,
    Green,
    Blue,
}

pub enum Shape {
    Circle(f64),
    Rect { width: f64, height: f64 },
}

impl Shape {
    pub fn circle(radius: f64) -> Shape {
        Shape::Circle(radius)
    }

    pub fn area(&self) -> f64 {
        match self {
            Shape::Circle(radius) => 3.0 * radius * radius,
            Shape::Rect { width, height } => width * height,
        }
    }
}

pub fn favorite() -> Color {
    Color::Green
}
