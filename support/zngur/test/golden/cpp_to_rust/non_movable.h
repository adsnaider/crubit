// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NON_MOVABLE_H_
#define THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NON_MOVABLE_H_

#include <cstdint>

namespace non_movable {

// Abstract: Rust only refers to it.
class Shape {
 public:
  virtual ~Shape() = default;
  virtual int32_t Area() const = 0;
};

// Not movable with `memcpy` (it has a destructor): Rust only refers to it, so
// its constructors are left out.
class Resource {
 public:
  explicit Resource(int32_t id) : id_(id) {}
  Resource(const Resource&) = delete;
  Resource& operator=(const Resource&) = delete;
  ~Resource() { id_ = -1; }

  int32_t Id() const { return id_; }
  static Resource* Create(int32_t id) { return new Resource(id); }

 private:
  int32_t id_;
};

int32_t AreaOf(const Shape& shape);

}  // namespace non_movable

#endif  // THIRD_PARTY_CRUBIT_SUPPORT_ZNGUR_TEST_GOLDEN_CPP_TO_RUST_NON_MOVABLE_H_
