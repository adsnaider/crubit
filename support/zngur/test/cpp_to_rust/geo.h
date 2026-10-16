// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_CPP_TO_RUST_GEO_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_CPP_TO_RUST_GEO_H_

#include <cstdint>
#include <memory>

namespace geo {

struct Point {
  Point() : x(0), y(0) {}
  Point(int32_t x, int32_t y) : x(x), y(y) {}

  int32_t Norm2() const { return x * x + y * y; }
  void Scale(int32_t factor) {
    x *= factor;
    y *= factor;
  }
  static Point Origin() { return Point(); }

  int32_t x;
  int32_t y;
};

// Not movable with `memcpy`, so Rust only refers to it.
class Counter {
 public:
  Counter() = default;
  Counter(const Counter&) = delete;
  Counter& operator=(const Counter&) = delete;
  virtual ~Counter() = default;

  virtual int32_t Next() { return ++count_; }

 private:
  int32_t count_ = 0;
};

int32_t Add(int32_t a, int32_t b);
std::unique_ptr<Point> MakePoint(int32_t x, int32_t y);
std::unique_ptr<Counter> MakeCounter();
int32_t NextOf(Counter& counter);

}  // namespace geo

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_CPP_TO_RUST_GEO_H_
