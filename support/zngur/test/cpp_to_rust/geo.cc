// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#include "support/zngur/test/cpp_to_rust/geo.h"

#include <cstdint>
#include <memory>

namespace geo {

int32_t Add(int32_t a, int32_t b) { return a + b; }

std::unique_ptr<Point> MakePoint(int32_t x, int32_t y) {
  return std::make_unique<Point>(x, y);
}

std::unique_ptr<Counter> MakeCounter() { return std::make_unique<Counter>(); }

int32_t NextOf(Counter& counter) { return counter.Next(); }

}  // namespace geo
