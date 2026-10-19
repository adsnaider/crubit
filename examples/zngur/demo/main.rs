// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Rust using the C++ library `shapes.h`, through crubit's generated `.zng`
//! file plus the hand-written `shapes_extra.zng`.

use shapes_zngur::shapes::{MakeTriangle, Point};
use shapes_zngur::std::make_canvas;
use zngur_lib::ZngCppConstruct;

fn main() {
    // Generated: a by-value C++ type, its constructors and its overloads.
    let mut p = Point::build((1, 2));
    p.Scale_2(10, 100);
    println!("[Rust] p = ({}, {})", p.x().get(), p.y().get());

    // Generated: a function returning a `std::unique_ptr`.
    let triangle = MakeTriangle();
    // SAFETY: `triangle` owns a `Polygon`, which lives as long as it does, and nothing else refers
    // to it meanwhile.
    let triangle = unsafe { triangle.get().as_mut() }.expect("null");
    triangle.Add(&p);
    println!("[Rust] the polygon has {} points", triangle.Size());

    // Added by hand (shapes_extra.zng, #1): `std::vector<Point>`, and the member
    // functions that crubit skipped because of it.
    let points = triangle.Points();
    for i in 0..points.size() {
        let point = points.at(i);
        println!("[Rust] point {i}: ({}, {})", point.x().get(), point.y().get());
    }
    let mut reversed = triangle.Reversed();
    reversed.push_back(&Point::build((7, 7)));
    println!("[Rust] reversed, plus one more: {} points", reversed.size());

    // Added by hand (shapes_extra.zng, #2): `std::make_unique<Canvas>()`.
    let canvas = make_canvas();
    // SAFETY: `canvas` owns a `Canvas`, which lives as long as it does, and nothing else refers to
    // it meanwhile.
    let canvas_ref = unsafe { canvas.get().as_mut() }.expect("null");
    for i in 0..points.size() {
        canvas_ref.Draw(points.at(i));
    }
    println!("[Rust] drew {} points", canvas_ref.Count());
    drop(canvas);
}
