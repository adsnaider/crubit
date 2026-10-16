# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

"""Build rules for bindings through zngur (https://github.com/HKalbasi/zngur).

crubit describes the API of a library in a zngur `.zng` file (`zng_from_rust`, `zng_from_cc`),
and zngur generates the bindings from it (`zngur_library`). The `.zng` file can be amended with
hand-written files that it merges (`extra_zng`), or replaced by a hand-written fork.

`rust_zngur_bindings` and `cc_zngur_bindings` do both steps.
"""

load("//support/zngur:providers.bzl", _ZngFileInfo = "ZngFileInfo", _ZngurInfo = "ZngurInfo")
load("//support/zngur:zng_from_cc.bzl", _zng_from_cc = "zng_from_cc")
load("//support/zngur:zng_from_rust.bzl", _zng_from_rust = "zng_from_rust")
load("//support/zngur:zngur_library.bzl", _zngur_label = "zngur_label", _zngur_library = "zngur_library")

ZngFileInfo = _ZngFileInfo
ZngurInfo = _ZngurInfo
zng_from_cc = _zng_from_cc
zng_from_rust = _zng_from_rust
zngur_library = _zngur_library

def rust_zngur_bindings(
        name,
        crate,
        deps = [],
        extra_zng = [],
        crate_name = None,
        visibility = None,
        testonly = False,
        tags = []):
    """C++ bindings for a Rust crate, through zngur.

    C++ code depends on `<name>`, and includes `<package>/<name>.zng.h`.

    Defines `<name>.zng` (the `.zng` file of the crate), and the targets of `zngur_library`.

    Args:
      name: The name of the C++ target.
      crate: The Rust crate.
      deps: The `rust_zngur_bindings` targets of crates whose types the crate's API uses.
      extra_zng: Hand-written `.zng` files that the `.zng` file merges, to amend the API.
      crate_name: The Rust crate of the generated Rust code. Defaults to `name`.
      visibility: The visibility of the targets.
      testonly: Whether the targets are test-only.
      tags: The tags of the targets.
    """
    common = {"testonly": testonly, "tags": tags}
    _zng_from_rust(
        name = name + "_zng",
        crate = crate,
        deps = deps,
        extra_zng = extra_zng,
        out = name + ".zng",
        visibility = visibility,
        **common
    )

    # For crates that use these bindings: the generated Rust code refers to the bound crate.
    native.alias(
        name = name + "_bound_crate",
        actual = crate,
        visibility = visibility,
        **common
    )
    _zngur_library(
        name = name,
        zng = ":" + name + "_zng",
        deps = deps,
        crate_name = crate_name,
        rust_deps = [crate] + [_zngur_label(dep, "_bound_crate") for dep in deps],
        visibility = visibility,
        **common
    )

def cc_zngur_bindings(
        name,
        lib,
        deps = [],
        extra_zng = [],
        crate_name = None,
        visibility = None,
        testonly = False,
        tags = []):
    """Rust bindings for a C++ library, through zngur.

    Rust code depends on `<name>_rs`, the crate `crate_name`.

    Defines `<name>.zng` (the `.zng` file of the library), and the targets of `zngur_library`.

    Args:
      name: The name of the target. `<name>_rs` is the Rust crate.
      lib: The C++ library.
      deps: The `cc_zngur_bindings` targets of libraries whose types the library's API uses.
      extra_zng: Hand-written `.zng` files that the `.zng` file merges, to amend the API.
      crate_name: The Rust crate of the bindings. Defaults to `name`.
      visibility: The visibility of the targets.
      testonly: Whether the targets are test-only.
      tags: The tags of the targets.
    """
    if crate_name == None:
        crate_name = name.replace("-", "_")
    common = {"testonly": testonly, "tags": tags}
    _zng_from_cc(
        name = name + "_zng",
        lib = lib,
        deps = deps,
        crate_name = crate_name,
        extra_zng = extra_zng,
        out = name + ".zng",
        visibility = visibility,
        **common
    )
    _zngur_library(
        name = name,
        zng = ":" + name + "_zng",
        deps = deps + [Label("//support/zngur:cpp_std")],
        crate_name = crate_name,
        cc_deps = [lib],
        visibility = visibility,
        **common
    )
