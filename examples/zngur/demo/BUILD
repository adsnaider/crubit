"""A demo of crubit as a zngur front end: generated bindings, amended by hand.

See README.md, and run `examples/zngur/demo/demo.sh`.
"""

load("@rules_cc//cc:cc_binary.bzl", "cc_binary")
load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library")
load("@rules_shell//shell:sh_test.bzl", "sh_test")
load("//support/zngur:defs.bzl", "cc_zngur_bindings", "rust_zngur_bindings", "zng_from_rust")

licenses(["notice"])

cc_library(
    name = "shapes",
    hdrs = ["shapes.h"],
    # crubit's default lifetimes make C++ references references to C++ objects, and member
    # functions methods.
    aspect_hints = ["//features:assume_lifetimes"],
)

# "Before": only what crubit generates.
cc_zngur_bindings(
    name = "shapes_generated",
    lib = ":shapes",
)

# "After": the generated `.zng` file, plus hand-written additions that it merges.
cc_zngur_bindings(
    name = "shapes_zngur",
    extra_zng = ["shapes_extra.zng"],
    lib = ":shapes",
)

rust_binary(
    name = "main",
    srcs = ["main.rs"],
    edition = "2021",
    deps = [
        ":shapes_zngur_rs",
        "//support/zngur:cpp_std_rs",
        "//support/zngur:zngur_lib",
    ],
)

sh_test(
    name = "main_test",
    size = "small",
    srcs = [":main"],
)

rust_library(
    name = "stats",
    srcs = ["stats.rs"],
    edition = "2021",
)

# "Before": only what crubit generates.
rust_zngur_bindings(
    name = "stats_generated",
    crate = ":stats",
)

# "After": plus hand-written additions.
rust_zngur_bindings(
    name = "stats_zngur",
    crate = ":stats",
    extra_zng = ["stats_extra.zng"],
)

cc_binary(
    name = "main_cc",
    srcs = ["main.cc"],
    deps = [":stats_zngur"],
)

sh_test(
    name = "main_cc_test",
    size = "small",
    srcs = [":main_cc"],
)

# Checked-in copies of the generated `.zng` files (for the slides), kept up to date by golden tests.
# They're in `generated/`, so that their paths differ from those of the generated files. (The copy
# of the `.zng` file of `stats` leaves out the rustc version and target.)
zng_from_rust(
    name = "stats_generated_golden_zng",
    crate = ":stats",
    is_golden_test = True,
)

[
    sh_test(
        name = name + "_golden_test",
        size = "small",
        srcs = ["//common:golden_test.sh"],
        args = [
            "$(rootpath generated/%s.zng)" % name,
            "$(rootpath %s)" % generated,
        ],
        data = [
            "generated/%s.zng" % name,
            generated,
            "//common:LICENSE_HEADER",
        ],
        tags = ["crubit_golden_test"],
    )
    for name, generated in [
        ("shapes", ":shapes_generated_zng"),
        ("stats", ":stats_generated_golden_zng"),
    ]
]
