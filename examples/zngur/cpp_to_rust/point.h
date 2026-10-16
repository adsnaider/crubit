// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_CPP_TO_RUST_POINT_H_
#define THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_CPP_TO_RUST_POINT_H_

#include <cstdint>

namespace geometry {

struct Point {
  Point() : x(0), y(0) {}
  Point(int32_t x, int32_t y) : x(x), y(y) {}

  int32_t Norm2() const { return x * x + y * y; }

  int32_t x;
  int32_t y;
};

}  // namespace geometry

#endif  // THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_CPP_TO_RUST_POINT_H_
