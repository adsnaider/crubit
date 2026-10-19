// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_FUNCTIONS_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_FUNCTIONS_H_

#include <cstddef>
#include <cstdint>

#include "support/annotations.h"

namespace functions {

int32_t Add(int32_t a, int32_t b);
double Mix(double a, float b);
bool IsPositive(int64_t value);
uint8_t Low(uint16_t value);
size_t Count();
void Nothing();

// `long long` is `i64` as a parameter or return value.
long long Square(long long value);

// Functions with C++ pointers or references are `unsafe`.
void Fill(int32_t* out, int32_t value);
int32_t Read(const int32_t& value);

// So are functions that crubit is told are unsafe.
CRUBIT_UNSAFE int32_t Unchecked(int32_t index);

// Functions that crubit is told are safe are, even if they take pointers (zngur warns about them,
// so the file silences the warning with `#allow(safe_raw_pointer)`).
CRUBIT_UNSAFE_MARK_SAFE void FillChecked(int32_t* out, int32_t value);

namespace inner {
int32_t Twice(int32_t value);
}  // namespace inner

}  // namespace functions

int32_t GlobalFunction();

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_FUNCTIONS_H_
