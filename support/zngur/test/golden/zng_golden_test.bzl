# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

"""Golden tests of the `.zng` files that crubit generates, and of the bindings zngur builds from them.

To update the goldens:

    bazelisk test --test_env=WRITE_GOLDENS=1 --strategy=TestRunner=local //support/zngur/test/golden/...
"""

load("@bazel_skylib//rules:build_test.bzl", "build_test")
load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_rust//rust:defs.bzl", "rust_library")
load("@rules_shell//shell:sh_test.bzl", "sh_test")
load("//support/zngur:defs.bzl", "zng_from_cc", "zng_from_rust", "zngur_library")

def _zng_golden_test(name, zng, build):
    sh_test(
        name = name + "_golden_test",
        size = "small",
        srcs = ["//common:golden_test.sh"],
        args = ["$(rootpath %s.zng)" % name, "$(rootpath %s)" % zng],
        data = ["//common:LICENSE_HEADER", name + ".zng", zng],
        tags = ["crubit_golden_test"],
    )
    if build:
        build_test(
            name = name + "_build_test",
            targets = [":" + name + "_zngur", ":" + name + "_zngur_rs"],
        )

def cc_zng_golden_test(name, deps = [], cc_deps = [], aspect_hints = [], build = True):
    """A golden test of the `.zng` file of the C++ library `<name>.h`.

    Defines `<name>` (the library), `<name>_zngur` (its bindings, crate `<name>_zngur`),
    `<name>_golden_test` (compares the `.zng` file with `<name>.zng`), and `<name>_build_test`
    (builds the bindings).

    Args:
      name: The name of the test, and of its header.
      deps: The tests whose libraries the library uses. The `.zng` file imports theirs.
      cc_deps: Other dependencies of the library.
      aspect_hints: The `aspect_hints` of the library, e.g. its crubit features
        (`//features:assume_lifetimes`), which decide how crubit reads C++ references.
      build: Whether to build the bindings. Only `False` with a comment about what's missing.
    """
    generated_tags = [] if build else ["manual"]
    cc_library(
        name = name,
        hdrs = [name + ".h"],
        deps = deps + cc_deps,
        aspect_hints = aspect_hints,
    )
    zng_from_cc(
        name = name + "_zngur_zng",
        lib = ":" + name,
        deps = [":" + dep + "_zngur" for dep in deps],
        crate_name = name + "_zngur",
        out = name + "_zngur.zng",
        is_golden_test = True,
    )
    zngur_library(
        name = name + "_zngur",
        zng = ":" + name + "_zngur_zng",
        deps = [":" + dep + "_zngur" for dep in deps] + ["//support/zngur:cpp_std"],
        crate_name = name + "_zngur",
        cc_deps = [":" + name],
        tags = generated_tags,
    )
    _zng_golden_test(name, ":" + name + "_zngur_zng", build)

def rust_zng_golden_test(name, deps = [], build = True):
    """A golden test of the `.zng` file of the Rust crate `<name>.rs`.

    Defines `<name>` (the crate), `<name>_zngur` (its bindings, crate `<name>_zngur`),
    `<name>_golden_test` (compares the `.zng` file with `<name>.zng`), and `<name>_build_test`
    (builds the bindings).

    Args:
      name: The name of the test, and of its crate.
      deps: The tests whose crates the crate uses. The `.zng` file imports theirs.
      build: Whether to build the bindings. Only `False` with a comment about what's missing.
    """
    generated_tags = [] if build else ["manual"]
    rust_library(
        name = name,
        srcs = [name + ".rs"],
        edition = "2021",
        deps = [":" + dep for dep in deps],
    )
    zng_from_rust(
        name = name + "_zngur_zng",
        crate = ":" + name,
        deps = [":" + dep + "_zngur" for dep in deps],
        out = name + "_zngur.zng",
        is_golden_test = True,
    )
    zngur_library(
        name = name + "_zngur",
        zng = ":" + name + "_zngur_zng",
        deps = [":" + dep + "_zngur" for dep in deps],
        crate_name = name + "_zngur",
        rust_deps = [":" + name] + [":" + dep for dep in deps],
        tags = generated_tags,
    )
    _zng_golden_test(name, ":" + name + "_zngur_zng", build)
