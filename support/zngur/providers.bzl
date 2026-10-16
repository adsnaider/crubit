# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

"""Providers of the zngur build rules."""

ZngFileInfo = provider(
    doc = """A `.zng` file that crubit generated (`zng_from_rust`, `zng_from_cc`).

The `.zng` file and the hand-written files that it merges are in the output tree, next to each
other, as zngur resolves `merge` and `import` paths relative to the file.""",
    fields = {
        "zng": "File: the `.zng` file.",
        "srcs": "list[File]: the hand-written `.zng` files that the file merges.",
        "transitive_files": "depset[File]: every `.zng` file that zngur reads to process the " +
                            "file: the file, the files it merges, and its imports, transitively.",
        "bound_rust_crate": "str|None: the Rust crate that the file describes (Rust to C++).",
        "bound_cc_target": "str|None: the label of the C++ target that the file describes " +
                           "(C++ to Rust).",
        "bound_cc_headers": "list[File]: the public headers of `bound_cc_target`.",
    },
)

ZngurInfo = provider(
    doc = """A zngur module (`zngur_library`): a `.zng` file and the bindings generated from it.

Other modules import the `.zng` file, to refer to its types.""",
    fields = {
        "zng": "File: the `.zng` file, in the output tree. The generated header is next to it, " +
               "as `<zng>.h`, so that the headers of importing modules can include it.",
        "header": "File: the generated C++ header.",
        "crate_name": "str: the Rust crate of the generated Rust code.",
        "transitive_files": "depset[File]: every `.zng` file that zngur reads to process the " +
                            "file: the file, the files it merges, and its imports, transitively.",
        "bound_rust_crate": "str|None: the Rust crate that the module describes (Rust to C++).",
        "bound_cc_target": "str|None: the label of the C++ target that the module describes " +
                           "(C++ to Rust).",
        "bound_cc_headers": "list[File]: the public headers of `bound_cc_target`, which " +
                            "`zng_from_cc` attributes to it.",
    },
)
