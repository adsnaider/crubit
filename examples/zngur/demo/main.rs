// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Rust using the C++ library `shapes.h`, through crubit's generated `.zng`
//! file plus the hand-written `shapes_extra.zng`.

use shapes_zngur::shapes::{MakeTriangle, Point};
use shapes_zngur::std::make_canvas;
use zngur_lib::CppCtor;

fn main() {
    // Generated: a by-value C++ type, its constructors and its overloads.
    let mut p = Point::new_2(1, 2).build();
    p.Scale_2(10, 100);
    println!("[Rust] p = ({}, {})", p.x().get(), p.y().get());

    // Generated: a function returning a `std::unique_ptr`.
    let mut triangle = MakeTriangle();
    // `get_mut()` borrows the owned `Polygon` uniquely (`None` if the `unique_ptr` is empty).
    let triangle = triangle.get_mut().expect("empty std::unique_ptr");
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
    reversed.push_back(&Point::new_2(7, 7).build());
    println!("[Rust] reversed, plus one more: {} points", reversed.size());

    // Added by hand (shapes_extra.zng, #2): `std::make_unique<Canvas>()`.
    let mut canvas = make_canvas();
    if let Some(canvas) = canvas.get_mut() {
        for i in 0..points.size() {
            canvas.Draw(points.at(i));
        }
        println!("[Rust] drew {} points", canvas.Count());
    }
    drop(canvas);
}
