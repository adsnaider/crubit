// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#ifndef CRUBIT_RS_BINDINGS_FROM_CC_SRC_CODE_GEN_H_
#define CRUBIT_RS_BINDINGS_FROM_CC_SRC_CODE_GEN_H_

#include <string>
#include <vector>

#include "absl/status/statusor.h"
#include "absl/strings/string_view.h"
#include "rs_bindings_from_cc/ir.h"
#include "rs_bindings_from_cc/profiling.h"

namespace crubit {

// Source code for generated bindings.
struct Bindings {
  // Rust source code.
  std::string rs_api;
  // C++ source code.
  std::string rs_api_impl;
  // Optional JSON error report.
  std::string error_report;
};

// Generates bindings from the given `IR`.
CRUBIT_PROFILE_NOINLINE absl::StatusOr<Bindings> GenerateBindings(
    IR ir, absl::string_view crubit_support_path_format,
    absl::string_view crubit_support_versioned_path_format,
    absl::string_view clang_format_exe_path, absl::string_view rustfmt_exe_path,
    absl::string_view rustfmt_config_path, bool generate_error_report,
    bool is_golden_test, bool kythe_annotations,
    absl::string_view kythe_default_corpus);

// Options of a zngur `.zng` file (see `generate_bindings/generate_zng.rs`).
struct ZngOptions {
  // The output path of the `.zng` file.
  std::string zng_out;
  // The Rust crate of the zngur bindings (empty for the target's crate name).
  std::string crate_name;
  // `target=path` pairs: the `.zng` files of dependency targets.
  std::vector<std::string> imports;
  // Hand-written `.zng` files that the generated file merges.
  std::vector<std::string> merges;
};

// Generates a zngur `.zng` file for the current target of `ir`, instead of
// bindings.
CRUBIT_PROFILE_NOINLINE absl::StatusOr<std::string> GenerateZng(
    IR ir, const ZngOptions& options, bool is_golden_test);

}  // namespace crubit

#endif  // CRUBIT_RS_BINDINGS_FROM_CC_SRC_CODE_GEN_H_
