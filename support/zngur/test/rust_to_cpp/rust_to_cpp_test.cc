// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

// Calls Rust crates through the bindings of `rust_zngur_bindings`.

#include <cstdint>

#include "gtest/gtest.h"
#include "support/zngur/test/rust_to_cpp/geo_zngur.zng.h"
#include "support/zngur/test/rust_to_cpp/shapes_zngur.zng.h"

namespace {

using ::rust::geo::Path;
using ::rust::geo::Point;
using ::rust::shapes::Segment;

TEST(RustToCppTest, FreeFunction) { EXPECT_EQ(::rust::geo::add(2, 3), 5); }

TEST(RustToCppTest, CopyType) {
  Point p = Point::new_(3, 4);
  EXPECT_EQ(p.norm2(), 25);
  p.scale(2);
  EXPECT_EQ(static_cast<int32_t>(p.x), 6);
  EXPECT_EQ(static_cast<int32_t>(p.y), 8);

  Point copy = p;
  copy.scale(0);
  EXPECT_EQ(p.norm2(), 100);
  EXPECT_EQ(copy.norm2(), 0);
  EXPECT_EQ(Point::default_().norm2(), 0);
}

TEST(RustToCppTest, TypeWithDestructor) {
  Path path = Path::default_();
  path.push(Point::new_(1, 2));
  path.push(Point::new_(3, 4));
  EXPECT_EQ(path.len(), 2u);
}

TEST(RustToCppTest, TypeOfDependency) {
  Segment segment = Segment::new_(Point::new_(1, 1), Point::new_(4, 5));
  EXPECT_EQ(segment.length2(), 25);
  EXPECT_EQ(static_cast<int32_t>(segment.to.x), 4);
}

TEST(RustToCppTest, ExtraZng) {
  auto quotient = ::rust::geo::checked_div(7, 2);
  ASSERT_TRUE(quotient.is_some());
  EXPECT_EQ(quotient.unwrap(), 3);
  EXPECT_FALSE(::rust::geo::checked_div(1, 0).is_some());
}

}  // namespace
