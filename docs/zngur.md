<!-- Part of the Crubit project, under the Apache License v2.0 with LLVM -->
<!-- Exceptions. See /LICENSE for license information. -->
<!-- SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception -->

# Bindings through zngur

crubit can be a front end for [zngur](https://github.com/HKalbasi/zngur), in
both directions. In this mode crubit only analyzes a library and describes its
API in a zngur `.zng` file. zngur then generates all of the bindings from that
file: the Rust code, the C++ header, and the C++ glue.

1.  crubit (`cc_bindings_from_rs --zng-out` or `rs_bindings_from_cc --zng_out`)
    reads a Rust crate or a C++ library, and writes a `.zng` file.
2.  The `.zng` file merges any hand-written `.zng` files that amend it.
3.  zngur generates the bindings from the `.zng` file: a Rust library and a C++
    library.

The `.zng` file is plain text, so you can read it, amend it with hand-written
files that it merges, or replace it with a fork that you check in. Everything
crubit can't describe yet is listed at the top of the file, in
`// skipped: <item>: <reason>` comments.

Working examples are in
[`examples/zngur`](https://github.com/google/crubit/tree/main/examples/zngur).

## Compared with crubit's own bindings

|                 | crubit's bindings         | Through zngur                  |
| --------------- | ------------------------- | ------------------------------ |
| Generated code  | crubit's own generators   | zngur, from the `.zng` file    |
| Amending        | Attributes in the source  | Hand-written or forked `.zng`  |
| C++ API of Rust | The crate's namespace     | zngur's `rust::my_crate::Foo`  |
| Rust API of C++ | Rust types with lifetimes | `c++::` types, via `CppRef`    |
| Coverage        | Wide                      | The subset below, rest skipped |

zngur's bindings follow zngur's documentation: see its book, in particular
"Calling C++ from Rust" and "Calling Rust from C++".

## Bazel rules

The rules are in `//support/zngur:defs.bzl`.

### C++ bindings for a Rust crate

```python
load("//support/zngur:defs.bzl", "rust_zngur_bindings")

rust_library(
    name = "geometry",
    srcs = ["geometry.rs"],
)

rust_zngur_bindings(
    name = "geometry_zngur",
    crate = ":geometry",
    # The `rust_zngur_bindings` of crates whose types `geometry`'s API uses.
    deps = [],
    # Hand-written additions to the `.zng` file.
    extra_zng = [],
)

cc_binary(
    name = "main",
    srcs = ["main.cc"],
    deps = [":geometry_zngur"],
)
```

C++ code includes `<package>/geometry_zngur.zng.h`, and uses the crate's items
under `rust::geometry::`.

### Rust bindings for a C++ library

```python
load("//support/zngur:defs.bzl", "cc_zngur_bindings")

cc_library(
    name = "geometry",
    hdrs = ["geometry.h"],
)

cc_zngur_bindings(
    name = "geometry_zngur",
    lib = ":geometry",
    # The `cc_zngur_bindings` of libraries whose types `geometry`'s API uses.
    deps = [],
    extra_zng = [],
)

rust_binary(
    name = "main",
    srcs = ["main.rs"],
    deps = [
        ":geometry_zngur_rs",
        "//support/zngur:cpp_std_rs",  # For `std::unique_ptr`.
        "//support/zngur:zngur_lib",  # For traits like `ZngCppConstruct`.
    ],
)
```

Rust code uses the crate `geometry_zngur` (the `crate_name` argument, by default
the target's name), with the library's types and functions at their C++ paths:
`geometry_zngur::geo::Point` for `geo::Point`.

### Targets

Both macros define:

-   `<name>.zng`, the `.zng` file (target `<name>_zng`).
-   `<name>`: the C++ side, which also carries the `.zng` file for the modules
    that import it.
-   `<name>_rs`: the Rust side.

They're made of three rules, which you can also use directly:

-   `zng_from_rust(crate, deps, extra_zng)` runs `cc_bindings_from_rs --zng-out`
    on a crate.
-   `zng_from_cc(lib, deps, crate_name, extra_zng)` runs
    `rs_bindings_from_cc --zng_out` on a library's public headers, with the
    library's compile flags.
-   `zngur_library(zng, srcs, deps, crate_name, rust_deps, cc_deps)` runs zngur
    on a `.zng` file, and builds its two sides.

### Dependencies

When a library's API uses the types of another library, its `.zng` file imports
that library's `.zng` file, and refers to the types through it. List the other
library's bindings in `deps`. Types from a library whose `.zng` file isn't
imported are skipped.

`std::unique_ptr<T>` is declared next to `T`, in the `.zng` file of `T`'s
library. Its home, `type<T> c++::std::unique_ptr<T>`, is in
`//support/zngur:cpp_std.zng`, which every `.zng` file of a C++ library imports.

## Amending the bindings

There are two ways:

-   **Additions:** `extra_zng = ["my_additions.zng"]`. The generated file
    `merge`s these files, which can declare more items or add to the generated
    ones. zngur combines declarations of the same item and reports conflicts.
    For example, to bind a function that returns `Option<i32>`, which crubit
    doesn't describe yet:

    ```zng
    type ::core::option::Option<i32> {
        #layout(size = 8, align = 4);
        wellknown_traits(Copy);

        fn is_some(&self) -> bool;
        fn unwrap(self) -> i32;
    }

    mod ::geometry {
        fn checked_div(i32, i32) -> ::core::option::Option<i32>;
    }
    ```

-   **A fork:** copy the generated `.zng` file into your package, edit it, and
    use it directly:
    `zngur_library(name = "geometry_zngur", zng = "geometry.zng", ...)`. It
    doesn't follow later changes to the library, but zngur still checks the
    layouts and signatures when it compiles the bindings.

Files that a `.zng` file merges must be in the same package.

## What crubit describes

### Rust crates

-   Structs, with their layout and public fields
    (`field x (offset = ..., type = ...)`).
-   Enums, with their layout, as opaque values.
-   Free functions, in their modules.
-   Inherent methods, with `self`, `&self` or `&mut self`. Methods whose names
    clash with a field get a `_` suffix.
-   `Copy`, `Clone`, `Default`, `Debug` and `Drop`.
-   Primitives, `bool`, `char` and `&str`.
-   Types of other crates, through imports.

Skipped: generic types and functions, types with lifetime parameters, fields of
reference type, traits, slices, `isize`, and items whose path contains a Rust
keyword.

### C++ libraries

-   Classes and structs (`type c++::ns::X`), with their layout:
    -   held by value (`#unsafe(trivially_relocatable)`) when crubit knows that
        they can be moved with `memcpy`;
    -   otherwise only referred to, through references (`CppRef`) and pointers
        (`CppPtr`).
-   Public fields (`field x` and `field x as x_mut`).
-   Member functions (static, `const` and non-`const`). Overloads get `_2`,
    `_3`, ... suffixes. They take `this` as a `CppRef`, a C++ reference.
-   Constructors of the types that Rust holds by value.
-   Free functions, in their namespaces.
-   Functions, member functions and constructors are `safe` unless they take C++
    pointers, which may be null or invalid, or are marked `CRUBIT_UNSAFE`. C++
    references are fine: a `CppRef` refers to a valid object.
-   `std::unique_ptr<T>`, for `T` of the same library.
-   Types of other libraries, through imports.

Skipped: enums, templates, type aliases, operators, nested types, unions,
functions in the global namespace, `char`, `void*`, pointers to pointers, and
`std::unique_ptr` of builtins.

## Without Bazel

The flags are:

-   `cc_bindings_from_rs --zng-out=<file>`, with `--zng-import=<crate>=<file>`
    for each crate whose types the API uses, and `--zng-merge=<file>` for each
    hand-written file. `--h-out` and `--rs-out` aren't used.
-   `rs_bindings_from_cc --zng_out=<file> --zng_crate_name=<crate>`, with
    `--zng_import=<target>=<file>,...` and `--zng_merge=<file>,...`. `--rs_out`
    and `--cc_out` aren't used.

Then run zngur on the file (`//support/zngur:zngur generate`, or zngur's own
command line). zngur resolves `import` and `merge` paths relative to the
importing file, and each generated header includes the headers of its imports
through those paths. So keep each `.zng` file next to its generated header.

## Limitations and future work

zngur's own gaps (lifetimes on `CppRef`, method-call syntax, inheritance, C++
enums, ...) are listed on the "Future work" page of the zngur book.

-   **Layouts are target-specific.** crubit writes the layouts of the target it
    runs for, and zngur checks them when the bindings compile. Hand-written
    `.zng` files, like `cpp_std.zng`, can pick layouts with zngur's (unstable)
    conditional compilation: `zngur_library` gives zngur the target's Rust
    `cfg`s (from `rustc --print cfg`), so `#if cfg!(target_pointer_width."64")`
    works.
-   **Generic Rust types** like `Option<T>` and `Vec<T>` aren't described yet.
    Today they can be added by hand, with `extra_zng`.
-   C++ enums, templates and operators aren't described yet.
-   The rules need rules_rust's
    `--@rules_rust//rust/settings:experimental_use_allocator_libraries_with_mangled_symbols=true`
    (in crubit's `.bazelrc`), so that C++ binaries that link Rust code get the
    allocator symbols.

## Tests

`support/zngur/test/golden` has golden tests of the `.zng` files in both
directions. Each one also builds the bindings that zngur generates. To update
the goldens:

```sh
bazelisk test --test_env=WRITE_GOLDENS=1 --strategy=TestRunner=local //support/zngur/test/golden/...
```
