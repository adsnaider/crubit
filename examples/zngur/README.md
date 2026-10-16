# Bindings through zngur

These examples use crubit as a front end for
[zngur](https://github.com/HKalbasi/zngur): crubit describes a library's API in
a zngur `.zng` file, and zngur generates the bindings from it. See
[docs/zngur.md](../../docs/zngur.md).

-   `cpp_to_rust/`: Rust calls a C++ library (`cc_zngur_bindings`).
-   `rust_to_cpp/`: C++ calls a Rust crate (`rust_zngur_bindings`).

Each example has:

1.  The library, `example.h` or `example.rs`, which uses the types of a second
    library (`point.h` or `point.rs`).
2.  `main.rs` or `main.cc`, which calls it. `bazel test //examples/zngur/...`
    runs it.
3.  `example_extra.zng`, a hand-written addition to the generated `.zng` file.
4.  `example_generated.zng`, a copy of the `.zng` file that crubit generates,
    for exposition. A golden test keeps it up to date.
