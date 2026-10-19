# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

"""`zng_from_cc`: generates a `.zng` file for a C++ library, with `rs_bindings_from_cc`."""

load("@bazel_tools//tools/build_defs/cc:action_names.bzl", "ACTION_NAMES")
load("@bazel_tools//tools/cpp:toolchain_utils.bzl", "find_cpp_toolchain", "use_cpp_toolchain")
load("@rules_cc//cc/common:cc_common.bzl", "cc_common")
load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("//features:crubit_feature_hint.bzl", "find_crubit_features")
load("//support/zngur:providers.bzl", "ZngFileInfo", "ZngurInfo")
load("//support/zngur:zngur_library.bzl", "stage_file")

_CrubitFeaturesInfo = provider(
    doc = "The crubit features of a C++ library, from its `aspect_hints`.",
    fields = {"features": "The list of enabled crubit features."},
)

def _crubit_features_aspect_impl(target, ctx):
    return [_CrubitFeaturesInfo(features = find_crubit_features(target, ctx))]

# Collects the crubit features of the library (e.g. `aspect_hints = ["//features:supported"]`),
# which decide how crubit reads its API: `assume_lifetimes` gives C++ references lifetimes, and so
# makes them references to C++ objects in the `.zng` file rather than raw pointers.
_crubit_features_aspect = aspect(
    implementation = _crubit_features_aspect_impl,
    attrs = {
        "_globally_enabled_features": attr.label(
            default = "//common/bazel_support:globally_enabled_features",
        ),
    },
)

# Compiler flags that don't apply to parsing headers (`-c`, `-o`, `-MF` and `-MT` take a value).
_IGNORED_FLAGS_WITH_VALUE = ["-c", "-o", "-MF", "-MT"]
_IGNORED_FLAG_PREFIXES = ["-MD", "-MMD", "-frandom-seed", "-Xactionanalyzer"]

def label_string(label):
    """`//foo:bar` for targets of the main repository, `@repo//foo:bar` otherwise."""
    if label.workspace_name == "":
        return "//{}:{}".format(label.package, label.name)
    return str(label)

# The libc++ headers that define the templates of `cpp_std.zng`, relative to its include directory.
# crubit attributes them to `cpp_std`, so that it knows where the templates are declared.
_CPP_STD_HEADERS = ["memory", "__memory/unique_ptr.h"]

def _cpp_std_headers(cc_toolchain):
    return [
        directory + "/" + header
        for directory in cc_toolchain.built_in_include_directories
        if directory.endswith("/c++/v1")
        for header in _CPP_STD_HEADERS
    ]

def _compile_flags(ctx, cc_toolchain, compilation_context):
    """Returns the flags to parse the headers of `compilation_context`, as the C++ compiler."""
    feature_configuration = cc_common.configure_features(
        ctx = ctx,
        cc_toolchain = cc_toolchain,
        requested_features = ctx.features,
        unsupported_features = ctx.disabled_features,
    )
    variables = cc_common.create_compile_variables(
        feature_configuration = feature_configuration,
        cc_toolchain = cc_toolchain,
        user_compile_flags = ctx.fragments.cpp.copts + ctx.fragments.cpp.cxxopts,
        include_directories = compilation_context.includes,
        quote_include_directories = compilation_context.quote_includes,
        system_include_directories = depset(
            cc_toolchain.built_in_include_directories,
            transitive = [compilation_context.system_includes],
        ),
        framework_include_directories = compilation_context.framework_includes,
        preprocessor_defines = depset(
            transitive = [compilation_context.defines, compilation_context.local_defines],
        ),
    )
    flags = cc_common.get_memory_inefficient_command_line(
        feature_configuration = feature_configuration,
        action_name = ACTION_NAMES.cpp_compile,
        variables = variables,
    )
    result = []
    skip_next = False
    for flag in flags:
        if skip_next:
            skip_next = False
            continue
        if flag in _IGNORED_FLAGS_WITH_VALUE:
            skip_next = True
            continue
        if any([flag.startswith(prefix) for prefix in _IGNORED_FLAG_PREFIXES]):
            continue
        result.append(flag)
    return result

def _zng_from_cc_impl(ctx):
    cc_toolchain = find_cpp_toolchain(ctx)
    compilation_context = ctx.attr.lib[CcInfo].compilation_context
    public_headers = compilation_context.direct_public_headers
    if not public_headers:
        fail("`{}` has no public headers".format(ctx.attr.lib.label))

    zng = ctx.actions.declare_file(ctx.attr.out or ctx.label.name + ".zng")
    srcs = [stage_file(ctx, src) for src in ctx.files.extra_zng]
    target = label_string(ctx.attr.lib.label)

    # Which headers belong to which target: crubit names the types of other targets through their
    # `.zng` files, and the standard library's through `cpp_std.zng`.
    cpp_std = ctx.attr._cpp_std[ZngurInfo]
    cpp_std_target = label_string(ctx.attr._cpp_std.label)
    target_args = [
        {"t": target, "h": [h.path for h in public_headers], "f": ctx.attr.lib[_CrubitFeaturesInfo].features},
        {"t": cpp_std_target, "h": _cpp_std_headers(cc_toolchain), "f": ["supported", "experimental"]},
    ]
    imports = [(cpp_std_target, cpp_std)]
    for dep in ctx.attr.deps:
        info = dep[ZngurInfo]
        if info.bound_cc_target == None:
            fail("`{}` doesn't describe a C++ target".format(dep.label))
        target_args.append({"t": info.bound_cc_target, "h": [h.path for h in info.bound_cc_headers]})
        imports.append((info.bound_cc_target, info))

    args = ctx.actions.args()
    args.add("--target=" + target)
    args.add("--target_args=" + json.encode(target_args))
    args.add("--public_headers=" + ",".join([h.path for h in public_headers]))
    args.add("--zng_out=" + zng.path)
    args.add("--zng_crate_name=" + ctx.attr.crate_name)
    args.add("--zng_import=" + ",".join(["{}={}".format(t, info.zng.path) for (t, info) in imports]))
    if srcs:
        args.add("--zng_merge=" + ",".join([src.path for src in srcs]))
    args.add("--crubit_support_path_format=\"support/{header}\"")
    if ctx.attr.is_golden_test:
        args.add("--is_golden_test")
    args.add("--")
    args.add_all(_compile_flags(ctx, cc_toolchain, compilation_context))

    # Clang sees the tool's own path as an input.
    args.add("-Wno-unused-command-line-argument")

    ctx.actions.run(
        executable = ctx.executable._rs_bindings_from_cc,
        arguments = [args],
        inputs = depset(
            srcs,
            transitive = [
                compilation_context.headers,
                cc_toolchain.all_files,
            ] + [info.transitive_files for (_, info) in imports],
        ),
        outputs = [zng],
        mnemonic = "ZngFromCc",
        progress_message = "Generating the .zng file of %{label}",
    )

    return [
        DefaultInfo(files = depset([zng])),
        ZngFileInfo(
            zng = zng,
            srcs = srcs,
            transitive_files = depset(
                [zng] + srcs,
                transitive = [info.transitive_files for (_, info) in imports],
            ),
            bound_rust_crate = None,
            bound_cc_target = target,
            bound_cc_headers = public_headers,
        ),
    ]

zng_from_cc = rule(
    implementation = _zng_from_cc_impl,
    doc = """Generates a zngur `.zng` file that describes the API of a C++ library.

The `.zng` file goes to `zngur_library`, which builds the bindings for Rust to use the library.
The module of the `.zng` file is the crate `crate_name`, which must be the `crate_name` of the
`zngur_library`.""",
    attrs = {
        "lib": attr.label(
            doc = "The C++ library, whose public headers (`hdrs`) are the API. Its crubit " +
                  "features (`aspect_hints`) apply, e.g. `//features:assume_lifetimes` for C++ " +
                  "references with lifetimes.",
            providers = [CcInfo],
            mandatory = True,
            aspects = [_crubit_features_aspect],
        ),
        "deps": attr.label_list(
            doc = "The `zngur_library` targets of libraries whose types the API uses. The `.zng` " +
                  "file imports their `.zng` files.",
            providers = [ZngurInfo],
        ),
        "crate_name": attr.string(
            doc = "The Rust crate of the bindings (`#crate`).",
            mandatory = True,
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
        "_cpp_std": attr.label(
            default = Label("//support/zngur:cpp_std"),
            providers = [ZngurInfo],
        ),
        "_rs_bindings_from_cc": attr.label(
            default = Label("//rs_bindings_from_cc:rs_bindings_from_cc"),
            executable = True,
            cfg = "exec",
        ),
    },
    toolchains = use_cpp_toolchain(),
    fragments = ["cpp"],
)
