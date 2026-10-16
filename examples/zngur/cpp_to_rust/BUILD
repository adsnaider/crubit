"""Rust calling a C++ library, through crubit's `.zng` file and zngur's bindings."""

load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_rust//rust:defs.bzl", "rust_binary")
load("@rules_shell//shell:sh_test.bzl", "sh_test")
load("//support/zngur:defs.bzl", "cc_zngur_bindings")

licenses(["notice"])

cc_library(
    name = "point_lib",
    hdrs = ["point.h"],
)

# Rust bindings for `point_lib`: the crate `point_lib_zngur`, in `:point_lib_zngur_rs`.
cc_zngur_bindings(
    name = "point_lib_zngur",
    lib = ":point_lib",
)

cc_library(
    name = "example_lib",
    hdrs = ["example.h"],
    deps = [":point_lib"],
)

cc_zngur_bindings(
    name = "example_lib_zngur",
    # Amends the generated `.zng` file.
    extra_zng = ["example_extra.zng"],
    lib = ":example_lib",
    # The API of `example_lib` uses types of `point_lib`.
    deps = [":point_lib_zngur"],
)

rust_binary(
    name = "main",
    srcs = ["main.rs"],
    edition = "2021",
    deps = [
        ":example_lib_zngur_rs",
        ":point_lib_zngur_rs",
        "//support/zngur:cpp_std_rs",
        "//support/zngur:zngur_lib",
    ],
)

sh_test(
    name = "main_test",
    size = "small",
    srcs = [":main"],
)

# Keeps the checked-in copy of the generated `.zng` file up to date.
sh_test(
    name = "example_golden_test",
    size = "small",
    srcs = ["//common:golden_test.sh"],
    args = [
        "$(rootpath example_generated.zng)",
        "$(rootpath :example_lib_zngur_zng)",
    ],
    data = [
        "example_generated.zng",
        ":example_lib_zngur_zng",
        "//common:LICENSE_HEADER",
    ],
    tags = ["crubit_golden_test"],
)
