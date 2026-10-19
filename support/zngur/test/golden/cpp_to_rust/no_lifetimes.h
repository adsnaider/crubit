// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NO_LIFETIMES_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NO_LIFETIMES_H_

#include <cstdint>

#include "support/annotations.h"

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
  // crubit's annotations decide whether functions are safe, even with raw receivers or
  // references.
  CRUBIT_UNSAFE_MARK_SAFE int32_t Peek() const { return count; }
  CRUBIT_UNSAFE int32_t Unchecked(int32_t value) const { return count + value; }

  int32_t count;
};

int32_t Read(const Counter& counter);

}  // namespace no_lifetimes

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NO_LIFETIMES_H_
