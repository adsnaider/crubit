// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

// Calls the Rust crate in `example.rs` through the bindings that zngur
// generates from crubit's `.zng` file.

#include <cassert>
#include <cstdint>
#include <iostream>

#include "examples/zngur/rust_to_cpp/example_zngur.zng.h"

int main() {
  using ::rust::example::Canvas;
  using ::rust::example::Rect;
  using ::rust::point::Point;

  // `Rect` is `Copy`, so C++ copies it. Its fields are accessible.
  Rect rect = Rect::new_(Point::new_(1, 2), Point::new_(7, 4));
  std::cout << "area = " << rect.area() << "\n";
  rect.translate(10, 10);
  std::cout << "min.norm2() = " << rect.min.norm2() << "\n";

  // From `example_extra.zng`.
  auto ratio = rect.aspect_ratio();
  assert(ratio.is_some());
  std::cout << "aspect ratio = " << ratio.unwrap() << "\n";

  // `Canvas` owns a `Vec`: C++ owns it, and drops it at the end of the scope.
  Canvas canvas = Canvas::default_();
  canvas.draw(rect);
  canvas.draw(Rect::new_(Point::new_(0, 0), Point::new_(3, 3)));
  int32_t total = canvas.total_area();
  std::cout << "total area = " << total << "\n";
  return total == 21 ? 0 : 1;
}
