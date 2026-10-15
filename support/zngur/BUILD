"""zngur support: building the bindings that zngur generates from `.zng` files."""

load("@rules_rust//rust:defs.bzl", "rust_binary")

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
