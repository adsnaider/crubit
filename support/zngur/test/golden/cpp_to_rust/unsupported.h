// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_UNSUPPORTED_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_UNSUPPORTED_H_

#include <cstddef>
#include <cstdint>
#include <memory>

namespace unsupported {

enum class Color { kRed, kGreen };

template <typename T>
struct Box {
  T value;
};

using Alias = int32_t;

struct Outer {
  struct Nested {
    int32_t x;
  };
  bool operator==(const Outer& other) const { return value == other.value; }
  Outer& operator+=(const Outer& other) {
    value += other.value;
    return *this;
  }

  int32_t value;
};

void TakesChar(char c);
void TakesVoidPointer(void* p);
void TakesPointerToPointer(int32_t** p);
long long* PointerToLongLong();
ptrdiff_t Difference();
Box<int32_t> MakeBox();
Color Favorite();
std::unique_ptr<int32_t> MakeInt();

}  // namespace unsupported

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_UNSUPPORTED_H_
