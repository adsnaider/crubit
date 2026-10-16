// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_CPP_TO_RUST_EXAMPLE_H_
#define THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_CPP_TO_RUST_EXAMPLE_H_

#include <cstdint>
#include <memory>

#include "examples/zngur/cpp_to_rust/point.h"

namespace shapes {

// Uses `geometry::Point`, from another library.
struct Rect {
  Rect(const geometry::Point& min, const geometry::Point& max)
      : min(min), max(max) {}

  int32_t Area() const { return (max.x - min.x) * (max.y - min.y); }
  void Translate(int32_t dx, int32_t dy) {
    min.x += dx;
    min.y += dy;
    max.x += dx;
    max.y += dy;
  }

  geometry::Point min;
  geometry::Point max;
};

// Rust only refers to it: it has a destructor and virtual functions, so it
// can't be moved with `memcpy`. Rust creates one with `std::make_unique` (see
// `example_extra.zng`).
class Canvas {
 public:
  virtual ~Canvas() = default;
  virtual void Draw(const Rect& rect) { total_area_ += rect.Area(); }
  int32_t TotalArea() const { return total_area_; }

 private:
  int32_t total_area_ = 0;
};

inline std::unique_ptr<Rect> MakeSquare(int32_t side) {
  return std::make_unique<Rect>(geometry::Point(), geometry::Point(side, side));
}

}  // namespace shapes

#endif  // THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_CPP_TO_RUST_EXAMPLE_H_
