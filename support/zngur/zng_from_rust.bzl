# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

"""`zng_from_rust`: generates a `.zng` file for a Rust crate, with `cc_bindings_from_rs`."""

load("@rules_rust//rust:rust_common.bzl", "CrateInfo")

# buildifier: disable=bzl-visibility
load("@rules_rust//rust/private:providers.bzl", "DepInfo")
load("//support/zngur:providers.bzl", "ZngFileInfo", "ZngurInfo")
load("//support/zngur:zngur_library.bzl", "stage_file")

def _crate_dirname(crate_info):
    return crate_info.output.dirname

def _zng_from_rust_impl(ctx):
    crate_info = ctx.attr.crate[CrateInfo]
    dep_info = ctx.attr.crate[DepInfo]
    rust_toolchain = ctx.toolchains["@rules_rust//rust:toolchain_type"]
    crubit = ctx.toolchains["//cc_bindings_from_rs/bazel_support:toolchain_type"]
    if crubit == None:
        fail("cc_bindings_from_rs is not available on this platform")
    crubit = crubit.cc_bindings_from_rs_toolchain_info.binary

    zng = ctx.actions.declare_file(ctx.attr.out or ctx.label.name + ".zng")
    srcs = [stage_file(ctx, src) for src in ctx.files.extra_zng]
    rmeta = crate_info.metadata or crate_info.output

    args = ctx.actions.args()
    args.add(zng, format = "--zng-out=%s")
    for dep in ctx.attr.deps:
        info = dep[ZngurInfo]
        if info.bound_rust_crate == None:
            fail("`{}` doesn't describe a Rust crate".format(dep.label))
        args.add("--zng-import={}={}".format(info.bound_rust_crate, info.zng.path))
    args.add_all(srcs, format_each = "--zng-merge=%s")
    args.add("--source-crate-name", crate_info.name)
    args.add("--extern={}={}".format(crate_info.name, rmeta.path))
    args.add("--enable-rmeta-interface")
    if ctx.attr.is_golden_test:
        args.add("--is-golden-test")

    # The arguments for rustc.
    args.add_all(rust_toolchain.rust_std_paths, before_each = "-L")
    args.add(rust_toolchain.target_flag_value, format = "--target=%s")
    args.add_all(
        dep_info.transitive_crates,
        map_each = _crate_dirname,
        uniquify = True,
        format_each = "-Ldependency=%s",
    )
    if rust_toolchain._toolchain_generated_sysroot:
        args.add(rust_toolchain.sysroot, format = "--sysroot=%s")

    # `cc_bindings_from_rs` links rustc's libraries dynamically.
    env = {}
    rustc_lib = rust_toolchain.rustc_lib.to_list()
    if rustc_lib:
        env["LD_LIBRARY_PATH"] = rustc_lib[0].dirname

    ctx.actions.run(
        executable = crubit,
        arguments = [args],
        inputs = depset(
            [rmeta] + srcs,
            transitive = [
                dep_info.transitive_crate_outputs,
                dep_info.transitive_metadata_outputs,
                rust_toolchain.rust_std,
                rust_toolchain.rustc_lib,
            ] + [dep[ZngurInfo].transitive_files for dep in ctx.attr.deps],
        ),
        outputs = [zng],
        env = env,
        mnemonic = "ZngFromRust",
        progress_message = "Generating the .zng file of %{label}",
        toolchain = "//cc_bindings_from_rs/bazel_support:toolchain_type",
    )

    return [
        DefaultInfo(files = depset([zng])),
        ZngFileInfo(
            zng = zng,
            srcs = srcs,
            transitive_files = depset(
                [zng] + srcs,
                transitive = [dep[ZngurInfo].transitive_files for dep in ctx.attr.deps],
            ),
            bound_rust_crate = crate_info.name,
            bound_cc_target = None,
            bound_cc_headers = [],
        ),
    ]

zng_from_rust = rule(
    implementation = _zng_from_rust_impl,
    doc = """Generates a zngur `.zng` file that describes the API of a Rust crate.

The `.zng` file goes to `zngur_library`, which builds the bindings for C++ to use the crate.""",
    attrs = {
        "crate": attr.label(
            doc = "The Rust crate.",
            providers = [CrateInfo, DepInfo],
            mandatory = True,
        ),
        "deps": attr.label_list(
            doc = "The `zngur_library` targets of crates whose types the API uses. The `.zng` " +
                  "file imports their `.zng` files.",
            providers = [ZngurInfo],
        ),
        "extra_zng": attr.label_list(
            doc = "Hand-written `.zng` files of this package that the `.zng` file merges, to " +
                  "amend the generated API.",
            allow_files = [".zng"],
        ),
        "out": attr.string(
            doc = "The name of the `.zng` file. Defaults to `<name>.zng`.",
        ),
        "is_golden_test": attr.bool(
            doc = "Whether to leave out information that varies between machines, for golden " +
                  "tests.",
        ),
    },
    toolchains = [
        "@rules_rust//rust:toolchain_type",
        config_common.toolchain_type("//cc_bindings_from_rs/bazel_support:toolchain_type", mandatory = False),
    ],
)
