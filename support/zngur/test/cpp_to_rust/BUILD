"""Tests of the zngur build rules: Rust bindings for C++ libraries (`cc_zngur_bindings`)."""

load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_rust//rust:defs.bzl", "rust_test")
load("//support/zngur:defs.bzl", "cc_zngur_bindings")

package(default_applicable_licenses = ["//:license"])

cc_library(
    name = "geo",
    testonly = True,
    srcs = ["geo.cc"],
    hdrs = ["geo.h"],
    # C++ references with default lifetimes are references to C++ objects in the `.zng` file.
    aspect_hints = ["//features:assume_lifetimes"],
)

cc_zngur_bindings(
    name = "geo_zngur",
    testonly = True,
    lib = ":geo",
)

cc_library(
    name = "shapes",
    testonly = True,
    hdrs = ["shapes.h"],
    aspect_hints = ["//features:assume_lifetimes"],
    deps = [":geo"],
)

cc_zngur_bindings(
    name = "shapes_zngur",
    testonly = True,
    extra_zng = ["shapes_extra.zng"],
    lib = ":shapes",
    deps = [":geo_zngur"],
)

rust_test(
    name = "cpp_to_rust_test",
    size = "small",
    srcs = ["cpp_to_rust_test.rs"],
    edition = "2021",
    deps = [
        ":geo_zngur_rs",
        ":shapes_zngur_rs",
        "//support/zngur:cpp_std_rs",
        "//support/zngur:zngur_lib",
    ],
)
