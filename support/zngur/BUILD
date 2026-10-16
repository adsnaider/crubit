"""zngur support: building the bindings that zngur generates from `.zng` files."""

load("@bazel_skylib//:bzl_library.bzl", "bzl_library")
load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_rust//rust:defs.bzl", "rust_binary")
load("//support/zngur:zngur_library.bzl", "zngur_library")

package(default_applicable_licenses = ["//:license"])

# A minimal zngur command line for the build rules (see zngur_cli.rs).
rust_binary(
    name = "zngur",
    srcs = ["cli/zngur_cli.rs"],
    edition = "2024",
    visibility = ["//visibility:public"],
    deps = [
        "@crate_index//:clap",  # v4
        "@crate_index//:zngur",
    ],
)

# The runtime of the Rust code that zngur generates.
alias(
    name = "zngur_lib",
    actual = "@crate_index//:zngur-lib",
    visibility = ["//visibility:public"],
)

# The runtime of the C++ code that zngur generates, which includes it as `zngur.h`.
genrule(
    name = "zngur_h_gen",
    outs = ["zngur.h"],
    cmd = "$(execpath :zngur) make-zng-header $@",
    tools = [":zngur"],
)

cc_library(
    name = "zngur_h",
    hdrs = ["zngur.h"],
    includes = ["."],
    visibility = ["//visibility:public"],
)

# The home of the C++ standard library templates in the `.zng` files of `zng_from_cc`.
# The layouts in cpp_std.zng are those of 64-bit targets.
zngur_library(
    name = "cpp_std",
    crate_name = "crubit_zngur_cpp_std",
    visibility = ["//visibility:public"],
    zng = "cpp_std.zng",
)

bzl_library(
    name = "defs_bzl",
    srcs = [
        "defs.bzl",
        "providers.bzl",
        "zng_from_cc.bzl",
        "zng_from_rust.bzl",
        "zngur_library.bzl",
    ],
    visibility = ["//visibility:public"],
)
