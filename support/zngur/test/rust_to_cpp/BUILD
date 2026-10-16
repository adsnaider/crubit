"""Tests of the zngur build rules: C++ bindings for Rust crates (`rust_zngur_bindings`)."""

load("@rules_cc//cc:cc_test.bzl", "cc_test")
load("@rules_rust//rust:defs.bzl", "rust_library")
load("//support/zngur:defs.bzl", "rust_zngur_bindings")

package(default_applicable_licenses = ["//:license"])

rust_library(
    name = "geo",
    testonly = True,
    srcs = ["geo.rs"],
    edition = "2021",
)

rust_zngur_bindings(
    name = "geo_zngur",
    testonly = True,
    crate = ":geo",
    extra_zng = ["geo_extra.zng"],
)

rust_library(
    name = "shapes",
    testonly = True,
    srcs = ["shapes.rs"],
    edition = "2021",
    deps = [":geo"],
)

rust_zngur_bindings(
    name = "shapes_zngur",
    testonly = True,
    crate = ":shapes",
    deps = [":geo_zngur"],
)

cc_test(
    name = "rust_to_cpp_test",
    size = "small",
    srcs = ["rust_to_cpp_test.cc"],
    deps = [
        ":geo_zngur",
        ":shapes_zngur",
        "@googletest//:gtest_main",
    ],
)
