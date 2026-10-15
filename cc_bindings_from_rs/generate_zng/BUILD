"""Generates zngur (https://github.com/HKalbasi/zngur) `.zng` files from Rust crates."""

load("@rules_rust//rust:defs.bzl", "rust_library")
load("//common:crubit_wrapper_macros_oss.bzl", "crubit_rust_test")

package(
    default_applicable_licenses = ["//:license"],
    default_visibility = ["//:__subpackages__"],
)

rust_library(
    name = "generate_zng",
    srcs = ["generate_zng.rs"],
    edition = "2024",
    deps = [
        "//cc_bindings_from_rs/generate_bindings",
        "//cc_bindings_from_rs/generate_bindings:query_compiler",
        "//cc_bindings_from_rs/generate_bindings/database",
        "//common:arc_anyhow",
        "//common:code_gen_utils",
        "@crate_index//:zngur-def",
        "@crate_index//:zngur-parser",
    ],
)

crubit_rust_test(
    name = "generate_zng_test",
    srcs = ["generate_zng_test.rs"],
    deps = [
        ":generate_zng",
        "//cc_bindings_from_rs:run_compiler_test_support",
        "//cc_bindings_from_rs/generate_bindings:test_helpers",
        "@crate_index//:tempfile",
    ],
)
