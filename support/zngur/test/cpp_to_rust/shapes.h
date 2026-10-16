// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_CPP_TO_RUST_SHAPES_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_CPP_TO_RUST_SHAPES_H_

#include <cstdint>

#include "support/zngur/test/cpp_to_rust/geo.h"

namespace shapes {

// Uses the types of another target (`geo`), through its `.zng` file.
struct Segment {
  Segment(const geo::Point& from, const geo::Point& to) : from(from), to(to) {}

  int32_t Length2() const {
    int32_t dx = to.x - from.x;
    int32_t dy = to.y - from.y;
    return dx * dx + dy * dy;
  }

  geo::Point from;
  geo::Point to;
};

inline Segment UnitSegment() { return Segment(geo::Point(), geo::Point(1, 0)); }

}  // namespace shapes

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_CPP_TO_RUST_SHAPES_H_
