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
| Rust API of C++ | Rust types with lifetimes | `c++::` types and references   |
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
    # Optional: crubit's default lifetimes for C++ references (see below).
    aspect_hints = ["//features:assume_lifetimes"],
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
        "//support/zngur:zngur_lib",  # For traits like `CppCtor`.
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
    library's compile flags and crubit features (from its `aspect_hints`).
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

### Example: creating objects with `std::make_unique`

Rust can't construct C++ objects that it can only refer to (types that can't be
moved with `memcpy`), because it can't hold them by value. It can own them on
the heap, though, through a `std::unique_ptr`. zngur can call instantiations of
function templates, so an `extra_zng` file can declare `std::make_unique` for
the type:

```zng
// The instantiation gets its layout and `get()` from `cpp_std.zng`.
type c++::std::unique_ptr<c++::shapes::Canvas> {}

mod c++::std {
    safe fn make_unique<c++::shapes::Canvas>() -> c++::std::unique_ptr<c++::shapes::Canvas> as make_canvas;
}
```

The parameters are those of the constructor to call. In Rust,
`<crate>::std::make_canvas()` returns the `std::unique_ptr<Canvas>`, which
destroys the object when it's dropped. `examples/zngur/cpp_to_rust` does this.

## What crubit describes

### Rust crates

-   Structs, with their layout and public fields
    (`field x (offset = ..., type = ...)`).
-   Enums, with their layout, as opaque values.
-   Free functions, in their modules.
-   Inherent methods, with `self`, `&self` or `&mut self`. Methods whose names
    clash with a field get a `_` suffix.
-   The lifetimes of functions and methods (`fn f<'a>(&'a str) -> &'a str`).
    Elided ones keep the names that crubit gives them (`'__anon1`, ...), so
    they're always declared.
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
    -   otherwise only referred to, through references and pointers.
-   Public fields: `field &const x` (a `&Cpp<i32, Const>` or `&T<Const>` in
    Rust) and, for non-`const` fields, `field &unique var x as x_mut` (a
    `&mut Cpp<i32>` or `&mut T`).
-   Member functions (static, `const` and non-`const`), which are Rust methods.
    Overloads get `_2`, `_3`, ... suffixes.
-   Constructors of the types that Rust holds by value: the copy constructor is
    `new_copy`, and the others are `new`, `new_2`, ... in declaration order
    (`Point::new_2(1, 2).build()`, `Point::new_copy(&p).build()`).
-   Free functions, in their namespaces.
-   C++ references, following crubit's own rule (see below).
-   Functions, member functions and constructors are `safe` or `unsafe` like in
    crubit's own bindings (see below).
-   `std::unique_ptr<T>`, for `T` of the same library.
-   Types of other libraries, through imports.

#### C++ references

A C++ reference with lifetime information is a reference to a C++ object in the
`.zng` file, whose lifetime the function declares:

-   `const T&` is `&'a const T` (`&'a T<Const>` in Rust, or
    `&'a Cpp<i32, Const>` for a builtin);
-   `T&` is `&'a unique var T` (`&'a mut T` / `&'a mut Cpp<i32>`), which matches
    crubit's `&mut`;
-   `this` is `&'a const self` (a `const` member function, a `&self` method) or
    `&'a unique var self` (a `&mut self` method).

The lifetime information comes from the same places as in crubit's own
bindings:

-   explicit annotations, like `const T& $a` (`support/lifetime_annotations.h`);
-   constructors, whose references get lifetimes like in crubit;
-   the `assume_lifetimes` feature (`aspect_hints =
    ["//features:assume_lifetimes"]`, also part of `//features:supported`),
    which gives references crubit's default lifetimes. With
    `assume_this_lifetimes`, only `this` gets one.

Lifetimes are always named and declared on the function, so the `.zng` file
never relies on elision. They keep crubit's names, including the ones it makes
up (`'__this`, `'__param_0`, ...). References that only have an elided lifetime
(e.g. those of constructors) get fresh names (`'a`, `'b`, ...).

Any other reference is raw: `const T&` / `T&` (a raw pointer in Rust), or a
`const self&` / `self&` receiver, which makes the member function an associated
function taking `this` as a raw pointer. A returned reference whose lifetime
crubit can't name is raw as well. C++ types with lifetime parameters are
skipped.

#### Safety

Whether a function, member function or constructor is `safe` follows crubit's
annotations, like in its own bindings. The `.zng` file decides, and zngur only
warns.

-   `CRUBIT_UNSAFE` functions are `unsafe`.
-   `CRUBIT_UNSAFE_MARK_SAFE` functions are `safe`, even if they take raw
    pointers. zngur warns about `safe` functions that take raw pointers, so the
    generated file then silences the warning with `#allow(safe_raw_pointer)`.
-   Other functions are `unsafe` if they take a raw pointer (a C++ pointer, a
    raw reference or a raw receiver), which may be null or invalid, and `safe`
    otherwise. References to C++ objects refer to valid objects, and returned
    raw pointers don't make a function `unsafe`. (Like in crubit, `this` doesn't
    count for constructors.)

Skipped: enums, templates, type aliases, operators, nested types, unions,
functions in the global namespace, `char`, `void*`, pointers to pointers,
`std::unique_ptr` of builtins, and C++ types with lifetime parameters.

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

zngur's own gaps (inheritance, C++ enums, ...) are listed on the "Future work"
page of the zngur book.

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
