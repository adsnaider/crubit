// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NO_LIFETIMES_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NO_LIFETIMES_H_

#include <cstdint>

namespace no_lifetimes {

// Without `assume_lifetimes` (or annotations), C++ references have no lifetimes: they're raw
// (`const T&` / `T&`, and `const self&` / `self&` receivers), and functions that take them are
// `unsafe`. Constructors infer the lifetimes of their references, like in crubit.
struct Counter {
  Counter() : count(0) {}

  int32_t Get() const { return count; }
  void Add(const int32_t& amount) { count += amount; }
  int32_t& Ref() { return count; }
  static int32_t Twice(int32_t value) { return value * 2; }

  int32_t count;
};

int32_t Read(const Counter& counter);

}  // namespace no_lifetimes

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NO_LIFETIMES_H_
