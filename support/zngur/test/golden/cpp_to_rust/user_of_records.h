// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_USER_OF_RECORDS_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_USER_OF_RECORDS_H_

#include <memory>

#include "support/zngur/test/golden/cpp_to_rust/records.h"
#include "support/zngur/test/golden/cpp_to_rust/unique_ptr.h"

namespace user_of_records {

// Types of other targets come from their `.zng` files.
struct Segment {
  records::Point from;
  records::Point to;
};

records::Point Midpoint(const Segment& segment);
std::unique_ptr<unique_ptr::Widget> Wrap(int32_t id);
std::unique_ptr<Segment> MakeSegment();

}  // namespace user_of_records

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_USER_OF_RECORDS_H_
