# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

"""`zngur_library`: builds the bindings that zngur generates from a `.zng` file."""

load("@rules_cc//cc:cc_library.bzl", "cc_library")
load("@rules_cc//cc/common:cc_common.bzl", "cc_common")
load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("@rules_rust//rust:defs.bzl", "rust_library")
load("//support/zngur:providers.bzl", "ZngFileInfo", "ZngurInfo")

def stage_file(ctx, file):
    """Returns `file` in the output tree, at its path in the current package.

    zngur resolves `import` and `merge` paths relative to the importing file, and the generated
    headers include each other with those paths, so all `.zng` files must be in the output tree,
    like the generated headers.

    Args:
      ctx: The rule context.
      file: A `.zng` file of the current package.

    Returns:
      `file`, if it's already in the output tree, otherwise a symlink to it.
    """
    if file.owner.workspace_name != ctx.label.workspace_name or file.owner.package != ctx.label.package:
        fail("`{}` must be in the package of `{}`".format(file.owner, ctx.label))
    if not file.is_source:
        return file
    staged = ctx.actions.declare_file(file.owner.name)
    ctx.actions.symlink(output = staged, target_file = file)
    return staged

def _zngur_generate_impl(ctx):
    if ZngFileInfo in ctx.attr.zng:
        if ctx.files.srcs:
            fail("`srcs` can only be given with a hand-written `zng` (use `extra_zng` of the " +
                 "rule that generates the `.zng` file)")
        info = ctx.attr.zng[ZngFileInfo]
        zng = info.zng
        if zng.owner.workspace_name != ctx.label.workspace_name or zng.owner.package != ctx.label.package:
            fail("`{}` must be in the package of `{}`".format(zng.owner, ctx.label))
        files = info.transitive_files
        bound_rust_crate = info.bound_rust_crate
        bound_cc_target = info.bound_cc_target
        bound_cc_headers = info.bound_cc_headers
    else:
        zng_files = ctx.attr.zng[DefaultInfo].files.to_list()
        if len(zng_files) != 1:
            fail("`zng` must be a single `.zng` file, got {}".format(zng_files))
        zng = stage_file(ctx, zng_files[0])
        files = depset([zng] + [stage_file(ctx, src) for src in ctx.files.srcs])
        bound_rust_crate = None
        bound_cc_target = None
        bound_cc_headers = []
    transitive_files = depset(
        transitive = [files] + [dep[ZngurInfo].transitive_files for dep in ctx.attr.deps],
    )

    h_file = ctx.actions.declare_file(zng.basename + ".h", sibling = zng)
    rs_file = ctx.actions.declare_file(zng.basename + ".rs", sibling = zng)
    cpp_file = ctx.actions.declare_file(zng.basename + ".cc", sibling = zng)
    root_file = ctx.actions.declare_file(ctx.label.name + "_lib.rs")

    args = ctx.actions.args()
    args.add("generate")
    args.add(zng)
    args.add("--h-file", h_file)
    args.add("--rs-file", rs_file)
    args.add("--cpp-file", cpp_file)
    args.add("--crate-name", ctx.attr.crate_name)
    args.add("--mangling-base", ctx.attr.crate_name)

    # The target's Rust `cfg`s, for conditional compilation in `.zng` files (`#if cfg!(...)`).
    rust_toolchain = ctx.toolchains["@rules_rust//rust:toolchain_type"]
    args.add("--rustc", rust_toolchain.rustc)
    args.add("--target", rust_toolchain.target_flag_value)
    ctx.actions.run(
        executable = ctx.executable._zngur,
        arguments = [args],
        inputs = depset(transitive = [transitive_files, rust_toolchain.rustc_lib]),
        tools = [rust_toolchain.rustc],
        outputs = [h_file, rs_file, cpp_file],
        mnemonic = "Zngur",
        progress_message = "Generating zngur bindings from %{input}",
    )

    ctx.actions.write(
        output = root_file,
        content = """\
// Automatically @generated crate root for the zngur bindings of `{zng}`.
#![allow(warnings)]

#[path = "{rs}"]
pub mod generated;
pub use generated::*;
""".format(
            zng = zng.short_path,
            # Relative to the crate root, which is in the directory of the package.
            rs = rs_file.short_path.removeprefix(root_file.short_path.rpartition("/")[0] + "/"),
        ),
    )

    return [
        DefaultInfo(files = depset([h_file, rs_file, cpp_file, root_file])),
        ZngurInfo(
            zng = zng,
            header = h_file,
            crate_name = ctx.attr.crate_name,
            transitive_files = transitive_files,
            bound_rust_crate = bound_rust_crate,
            bound_cc_target = bound_cc_target,
            bound_cc_headers = bound_cc_headers,
        ),
        OutputGroupInfo(
            zngur_h = depset([h_file]),
            zngur_rs = depset([root_file, rs_file]),
            zngur_root = depset([root_file]),
            zngur_cc = depset([cpp_file]),
        ),
    ]

_zngur_generate = rule(
    implementation = _zngur_generate_impl,
    doc = "Runs zngur on a `.zng` file.",
    attrs = {
        "zng": attr.label(
            doc = "The `.zng` file: a hand-written file, or a `zng_from_rust`/`zng_from_cc` target.",
            allow_files = [".zng"],
            mandatory = True,
        ),
        "srcs": attr.label_list(
            doc = "Hand-written `.zng` files that `zng` merges.",
            allow_files = [".zng"],
        ),
        "deps": attr.label_list(
            doc = "The zngur modules that `zng` imports.",
            providers = [ZngurInfo],
        ),
        "crate_name": attr.string(
            doc = "The Rust crate of the generated Rust code.",
            mandatory = True,
        ),
        "_zngur": attr.label(
            default = Label("//support/zngur:zngur"),
            executable = True,
            cfg = "exec",
        ),
    },
    toolchains = ["@rules_rust//rust:toolchain_type"],
)

def _zngur_library_impl(ctx):
    cc_info = cc_common.merge_cc_infos(
        direct_cc_infos = [ctx.attr.glue[CcInfo]],
        cc_infos = [ctx.attr.rust[CcInfo]],
    )
    return [
        # Building the target builds both sides.
        DefaultInfo(files = depset(
            [ctx.attr.generated[ZngurInfo].header],
            transitive = [ctx.attr.glue[DefaultInfo].files, ctx.attr.rust[DefaultInfo].files],
        )),
        cc_info,
        ctx.attr.generated[ZngurInfo],
    ]

_zngur_library = rule(
    implementation = _zngur_library_impl,
    doc = "The C++ side of a zngur module, with its `ZngurInfo` for importing modules.",
    attrs = {
        "generated": attr.label(providers = [ZngurInfo], mandatory = True),
        "glue": attr.label(providers = [CcInfo], mandatory = True),
        "rust": attr.label(providers = [CcInfo], mandatory = True),
    },
)

# buildifier: disable=unnamed-macro
def zngur_label(label, suffix):
    """Returns the label of the target `label + suffix`.

    Args:
      label: A label, or a label string relative to the current package (`//foo:bar`, `//foo`,
        `:bar`, ...).
      suffix: The suffix, e.g. `_rs`.

    Returns:
      The label with `suffix`, e.g. `//foo:bar_rs` or `//foo:foo_rs`.
    """
    label = native.package_relative_label(label)
    return label.same_package_label(label.name + suffix)

def zngur_library(
        name,
        zng,
        srcs = [],
        deps = [],
        crate_name = None,
        rust_deps = [],
        cc_deps = [],
        visibility = None,
        testonly = False,
        tags = []):
    """Builds the bindings that zngur generates from a `.zng` file.

    Defines:
      * `<name>`: the C++ side, for C++ code to include `<zng>.h` and call Rust, or to implement
        the C++ functions that Rust calls. Other zngur modules also depend on it, to import `zng`.
      * `<name>_rs`: the Rust side (crate `crate_name`), for Rust code to call C++.

    Args:
      name: The name of the C++ target.
      zng: The `.zng` file: a `zng_from_rust`/`zng_from_cc` target, or a hand-written file (for
        example a fork of a generated file).
      srcs: The hand-written `.zng` files that a hand-written `zng` merges.
      deps: The `zngur_library` targets of the modules that `zng` imports.
      crate_name: The Rust crate of `<name>_rs`. Defaults to `name`.
      rust_deps: Extra dependencies of the Rust side, e.g. the Rust crates that `zng` refers to.
      cc_deps: Extra dependencies of the C++ side, e.g. the C++ libraries that `zng` refers to.
      visibility: The visibility of the targets.
      testonly: Whether the targets are test-only.
      tags: The tags of the targets.
    """
    if crate_name == None:
        crate_name = name.replace("-", "_")
    common = {"testonly": testonly, "tags": tags}
    _zngur_generate(
        name = name + "_zngur",
        zng = zng,
        srcs = srcs,
        deps = deps,
        crate_name = crate_name,
        visibility = ["//visibility:private"],
        **common
    )
    for group in ["h", "rs", "root", "cc"]:
        native.filegroup(
            name = name + "_zngur_" + group,
            srcs = [":" + name + "_zngur"],
            output_group = "zngur_" + group,
            visibility = ["//visibility:private"],
            **common
        )

    # The C++ side: the generated header, and the C++ functions that Rust calls.
    cc_library(
        name = name + "_glue",
        hdrs = [":" + name + "_zngur_h"],
        srcs = [":" + name + "_zngur_cc"],
        deps = [Label("//support/zngur:zngur_h")] + deps + cc_deps,
        visibility = ["//visibility:private"],
        **common
    )

    # The Rust side: the generated Rust code, and the Rust functions that C++ calls.
    rust_library(
        name = name + "_rs",
        crate_name = crate_name,
        crate_root = ":" + name + "_zngur_root",
        srcs = [":" + name + "_zngur_rs"],
        edition = "2021",
        deps = [Label("//support/zngur:zngur_lib")] +
               [zngur_label(dep, "_rs") for dep in deps] + rust_deps,
        link_deps = [":" + name + "_glue"],
        visibility = visibility,
        **common
    )

    _zngur_library(
        name = name,
        generated = ":" + name + "_zngur",
        glue = ":" + name + "_glue",
        rust = ":" + name + "_rs",
        visibility = visibility,
        **common
    )
