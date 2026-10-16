// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

// C++ using the Rust crate `stats.rs`, through crubit's generated `.zng` file
// plus the hand-written `stats_extra.zng`.

#include <iostream>

#include "examples/zngur/demo/stats_zngur.zng.h"

int main() {
  // Generated: a Rust struct, its fields and methods.
  ::rust::stats::Summary summary = ::rust::stats::Summary::default_();
  for (int32_t value : {3, 4, 8}) {
    summary.add(value);
  }
  std::cout << "[C++] count = " << static_cast<int32_t>(summary.count)
            << ", total = " << static_cast<int64_t>(summary.total) << std::endl;

  // Added by hand (stats_extra.zng, #1): `Summary::mean`, which returns an
  // `Option<f64>`.
  auto mean = summary.mean();
  if (mean.is_some()) {
    std::cout << "[C++] mean = " << mean.unwrap() << std::endl;
  }

  // Added by hand (stats_extra.zng, #2): the trait `Shape`, through a
  // `Box<dyn Shape>`.
  auto square = ::rust::stats::make_square(1.5);
  std::cout << "[C++] square area = " << square.area() << std::endl;
  return 0;
}
