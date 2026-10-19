// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_LIFETIMES_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_LIFETIMES_H_

#include <cstdint>

#include "support/lifetime_annotations.h"

namespace lifetimes {

struct Holder {
  int32_t value;
};

// References with lifetime annotations are references to C++ objects, which crubit's rules make
// `&'a const T` for `const T&` and `&'a unique var T` for `T&`.
const int32_t& $a Value(const Holder& $a holder);
int32_t& $a ValueMut(Holder& $a holder);
const Holder& $a Pick(const Holder& $a first, const Holder& $b second);
const Holder& $static Default();

// A reference without an annotation is raw, so the function is `unsafe`.
void Reset(Holder& holder);

}  // namespace lifetimes

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_LIFETIMES_H_
