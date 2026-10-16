// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Calls the C++ library in `example.h` through the bindings that zngur generates from crubit's
//! `.zng` file.

use crubit_zngur_cpp_std::std::unique_ptr;
use example_lib_zngur::shapes::{Canvas, MakeCanvas, MakeSquare, Rect};
use point_lib_zngur::geometry::Point;
use zngur_lib::ZngCppConstruct;

fn main() {
    // `Point` and `Rect` are held by value in Rust.
    let min = Point::build((1, 2));
    let max = Point::build((4, 6));
    let mut rect = unsafe { Rect::build_unchecked((min.as_ref(), max.as_ref())) };

    // Member functions take `this` as a C++ pointer, so they're `unsafe`.
    unsafe {
        println!("area = {}", Rect::Area(rect.as_ref()));
        Rect::Translate(rect.as_mut(), 10, 10);
        // A field accessor, and a member function of a type from another library.
        println!("min.Norm2() = {}", Point::Norm2(Rect::min(rect.as_ref())));
        // From `example_extra.zng`.
        assert_eq!(Rect::area(rect.as_ref()), 12);
    }

    // `std::unique_ptr` is held by value too, and `get()` gives the object.
    let square = MakeSquare(3);
    let canvas = MakeCanvas();
    unsafe {
        let square = unique_ptr::<Rect>::get(square.as_ref());
        let canvas = unique_ptr::<Canvas>::get(canvas.as_ref());
        Canvas::Draw(canvas, square.into());
        Canvas::Draw(canvas, rect.as_ref());
        println!("total area = {}", Canvas::TotalArea(canvas.into()));
        assert_eq!(Canvas::TotalArea(canvas.into()), 21);
    }
}
