// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_UNIQUE_PTR_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_UNIQUE_PTR_H_

#include <cstdint>
#include <memory>

namespace unique_ptr {

struct Widget {
  int32_t id;
};

// `std::unique_ptr<Widget>` is declared in this file, next to `Widget`.
std::unique_ptr<Widget> MakeWidget(int32_t id);
void Consume(std::unique_ptr<Widget> widget);
int32_t IdOf(const std::unique_ptr<Widget>& widget);

}  // namespace unique_ptr

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_UNIQUE_PTR_H_
