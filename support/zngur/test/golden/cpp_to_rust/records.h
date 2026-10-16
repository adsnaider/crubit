// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_RECORDS_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_RECORDS_H_

#include <cstdint>

namespace records {

// Movable with `memcpy`: Rust holds it by value.
struct Point {
  Point() : x(0), y(0) {}
  Point(int32_t x, int32_t y) : x(x), y(y) {}

  int32_t Norm2() const { return x * x + y * y; }
  void Scale(int32_t factor) { Scale(factor, factor); }
  void Scale(int32_t factor_x, int32_t factor_y) {
    x *= factor_x;
    y *= factor_y;
  }
  static Point Origin() { return Point(); }

  int32_t x;
  int32_t y;
};

// C++ names that are Rust keywords.
struct Keywords {
  int32_t Use() const { return type; }
  void move(int32_t to) { match = to; }

  int32_t type;
  int32_t match;
};

// Only public fields are accessible.
class WithPrivateField {
 public:
  explicit WithPrivateField(int64_t value) : value_(value) {}
  int64_t Value() const { return value_; }

 private:
  int64_t value_;
};

namespace nested {

struct Inner {
  const Point* origin;
  double weight;
};

}  // namespace nested

}  // namespace records

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_RECORDS_H_
