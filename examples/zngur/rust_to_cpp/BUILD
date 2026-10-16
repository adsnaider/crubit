"""C++ calling a Rust crate, through crubit's `.zng` file and zngur's bindings."""

load("@rules_cc//cc:cc_binary.bzl", "cc_binary")
load("@rules_rust//rust:defs.bzl", "rust_library")
load("@rules_shell//shell:sh_test.bzl", "sh_test")
load("//support/zngur:defs.bzl", "rust_zngur_bindings", "zng_from_rust")

licenses(["notice"])

rust_library(
    name = "point",
    srcs = ["point.rs"],
    edition = "2021",
)

# C++ bindings for `point`: include `examples/zngur/rust_to_cpp/point_zngur.zng.h`.
rust_zngur_bindings(
    name = "point_zngur",
    crate = ":point",
)

rust_library(
    name = "example",
    srcs = ["example.rs"],
    edition = "2021",
    deps = [":point"],
)

rust_zngur_bindings(
    name = "example_zngur",
    crate = ":example",
    # Amends the generated `.zng` file.
    extra_zng = ["example_extra.zng"],
    # The API of `example` uses types of `point`.
    deps = [":point_zngur"],
)

cc_binary(
    name = "main",
    srcs = ["main.cc"],
    deps = [
        ":example_zngur",
        ":point_zngur",
    ],
)

sh_test(
    name = "main_test",
    size = "small",
    srcs = [":main"],
)

# Keeps the checked-in copy of the generated `.zng` file up to date. (This copy leaves out the
# rustc version and target, which the `.zng` file of `example_zngur` records.)
zng_from_rust(
    name = "example_golden_zng",
    crate = ":example",
    extra_zng = ["example_extra.zng"],
    is_golden_test = True,
    deps = [":point_zngur"],
)

sh_test(
    name = "example_golden_test",
    size = "small",
    srcs = ["//common:golden_test.sh"],
    args = [
        "$(rootpath example_generated.zng)",
        "$(rootpath :example_golden_zng)",
    ],
    data = [
        "example_generated.zng",
        ":example_golden_zng",
        "//common:LICENSE_HEADER",
    ],
    tags = ["crubit_golden_test"],
)
