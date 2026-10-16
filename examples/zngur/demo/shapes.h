// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_DEMO_SHAPES_H_
#define THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_DEMO_SHAPES_H_

#include <cstdint>
#include <iostream>
#include <memory>
#include <utility>
#include <vector>

namespace shapes {

// Movable with `memcpy`: Rust holds it by value.
struct Point {
  Point() : x(0), y(0) {}
  Point(int32_t x, int32_t y) : x(x), y(y) {}

  void Scale(int32_t factor) { Scale(factor, factor); }
  void Scale(int32_t factor_x, int32_t factor_y) {
    x *= factor_x;
    y *= factor_y;
  }

  int32_t x;
  int32_t y;
};

// Uses `std::vector<Point>`, which crubit doesn't describe: `Points()` and
// `Reversed()` are skipped in the generated `.zng` file.
class Polygon {
 public:
  void Add(const Point& point) { points_.push_back(point); }
  int32_t Size() const { return static_cast<int32_t>(points_.size()); }
  const std::vector<Point>& Points() const { return points_; }
  std::vector<Point> Reversed() const {
    return std::vector<Point>(points_.rbegin(), points_.rend());
  }

 private:
  std::vector<Point> points_;
};

// Not movable with `memcpy` (virtual functions and a destructor), so Rust can
// only refer to it, and can't create one with the generated bindings.
class Canvas {
 public:
  Canvas() { std::cout << "[C++] Canvas created" << std::endl; }
  Canvas(const Canvas&) = delete;
  Canvas& operator=(const Canvas&) = delete;
  virtual ~Canvas() { std::cout << "[C++] Canvas destroyed" << std::endl; }

  virtual void Draw(const Point& point) {
    std::cout << "[C++] Drawing (" << point.x << ", " << point.y << ")"
              << std::endl;
    ++count_;
  }
  int32_t Count() const { return count_; }

 private:
  int32_t count_ = 0;
};

inline std::unique_ptr<Polygon> MakeTriangle() {
  auto triangle = std::make_unique<Polygon>();
  triangle->Add(Point(0, 0));
  triangle->Add(Point(4, 0));
  triangle->Add(Point(0, 3));
  return triangle;
}

}  // namespace shapes

#endif  // THIRD_PARTY_CRUBIT_EXAMPLES_ZNGUR_DEMO_SHAPES_H_
