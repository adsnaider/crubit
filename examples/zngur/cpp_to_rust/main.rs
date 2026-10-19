// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Calls the C++ library in `example.h` through the bindings that zngur generates from crubit's
//! `.zng` file.

use example_lib_zngur::shapes::{MakeSquare, Rect};
use example_lib_zngur::std::make_canvas;
use point_lib_zngur::geometry::Point;
use zngur_lib::CppCtor;

fn main() {
    // `Point` and `Rect` are held by value in Rust.
    let min = Point::new_2(1, 2).build();
    let max = Point::new_2(4, 6).build();
    // A C++ `const Point&` parameter takes a `&Point` (or a `&Point<Const>`).
    let mut rect = Rect::new(&min, &max).build();

    // Member functions are methods: `&self` for `const` member functions, and `&mut self` for
    // the others, which crubit takes as unique references.
    println!("area = {}", rect.Area());
    rect.Translate(10, 10);
    // A field accessor, and a member function of a type from another library.
    println!("min.Norm2() = {}", rect.min().Norm2());
    // From `example_extra.zng`.
    assert_eq!(rect.area(), 12);

    // `std::unique_ptr` is held by value too, and `get()` gives a C++ pointer to the object.
    let square = MakeSquare(3);
    // From `example_extra.zng`: `std::make_unique<Canvas>()`.
    let canvas = make_canvas();
    // SAFETY: Each `unique_ptr` owns an object, which lives as long as it does, and nothing else
    // refers to the canvas meanwhile.
    let (square, canvas) =
        unsafe { (square.get().as_ref().expect("null"), canvas.get().as_mut().expect("null")) };
    canvas.Draw(square);
    canvas.Draw(&rect);
    println!("total area = {}", canvas.TotalArea());
    assert_eq!(canvas.TotalArea(), 21);
}
