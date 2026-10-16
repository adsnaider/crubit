// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Rust using the C++ library `shapes.h`, through crubit's generated `.zng`
//! file plus the hand-written `shapes_extra.zng`.

use crubit_zngur_cpp_std::std::unique_ptr;
use shapes_zngur::shapes::{Canvas, MakeTriangle, Point, Polygon};
use shapes_zngur::std::{make_canvas, vector};
use zngur_lib::ZngCppConstruct;

fn main() {
    // Generated: a by-value C++ type, its constructors and its overloads.
    let mut p = Point::build((1, 2));
    Point::Scale_2(p.as_mut(), 10, 100);
    println!("[Rust] p = ({}, {})", Point::x(p.as_ref()).read(), Point::y(p.as_ref()).read());

    // Generated: a function returning a `std::unique_ptr`.
    let triangle = MakeTriangle();
    // SAFETY: `triangle` owns a `Polygon`, which lives as long as it does.
    let triangle =
        unsafe { unique_ptr::<Polygon>::get(triangle.as_ref()).as_cpp_ref() }.expect("null");
    Polygon::Add(triangle, p.as_ref());
    println!("[Rust] the polygon has {} points", Polygon::Size(triangle.as_const()));

    // Added by hand (shapes_extra.zng, #1): `std::vector<Point>`, and the member
    // functions that crubit skipped because of it.
    let points = Polygon::Points(triangle.as_const());
    for i in 0..vector::<Point>::size(points) {
        let point = vector::<Point>::at(points, i);
        println!("[Rust] point {i}: ({}, {})", Point::x(point).read(), Point::y(point).read());
    }
    let mut reversed = Polygon::Reversed(triangle.as_const());
    vector::<Point>::push_back(reversed.as_mut(), Point::build((7, 7)).as_ref());
    println!("[Rust] reversed, plus one more: {} points", vector::<Point>::size(reversed.as_ref()));

    // Added by hand (shapes_extra.zng, #2): `std::make_unique<Canvas>()`.
    let canvas = make_canvas();
    // SAFETY: `canvas` owns a `Canvas`, which lives as long as it does.
    let canvas_ref =
        unsafe { unique_ptr::<Canvas>::get(canvas.as_ref()).as_cpp_ref() }.expect("null");
    for i in 0..vector::<Point>::size(points) {
        Canvas::Draw(canvas_ref, vector::<Point>::at(points, i));
    }
    println!("[Rust] drew {} points", Canvas::Count(canvas_ref.as_const()));
    drop(canvas);
}
