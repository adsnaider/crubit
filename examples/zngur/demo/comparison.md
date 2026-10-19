# Compared with crubit's own bindings

These numbers are for `shapes.h` in this directory, on x86_64 Linux, at the
commit that last updated them. The comparison is about where the work happens. It
doesn't measure the quality of the two kinds of bindings.

## What crubit writes for `shapes.h`

|                    | crubit's own bindings             | crubit as a zngur front end                    |
| ------------------ | --------------------------------- | ---------------------------------------------- |
| crubit writes      | the bindings themselves (Rust, and C++ thunks) | a 56-line (2.7 KB) `.zng` file |
| Reviewable API     | buried in generated Rust          | the `.zng` file reads like an API spec         |
| Overloads (`Scale`) | skipped ("Cannot generate bindings for overloaded function") | `Scale` and `Scale_2` |
| `Points()` / `Reversed()` (`std::vector<Point>`) | skipped | skipped, and brought back with 15 lines of `shapes_extra.zng` |
| Creating a `Canvas` from Rust | `Ctor`-based construction | `std::make_unique`, added in `shapes_extra.zng` |
| Unsupported items  | `// error:` comments in the generated Rust | `// skipped: <item>: <reason>` at the top of the `.zng` file |
| Fixing a skipped item | change crubit, or write FFI by hand | add a few lines to `extra_zng`, or fork the `.zng` file |

crubit's own bindings do better in places. They expose C++ fields as real Rust
fields, and they construct non-movable types in place with `Ctor`. Both follow
crubit's lifetime rules for references (here, with `assume_lifetimes`).

## How much code is generated

zngur generates more code than crubit does, about 4 to 5 times as much for
this header. Both sides below are formatted the same way (rustfmt, and
`clang-format --style=Google`); crubit's own output is written unformatted, a
whole item per line.

|            | crubit's own bindings | crubit + zngur |
| ---------- | --------------------- | -------------- |
| Rust       | 449 lines (21 KB)     | 846 lines (27 KB) |
| C++ source | 137 lines (4.5 KB)    | 243 lines (11 KB) |
| C++ header | (none: it uses the library's own header) | 1,718 lines (61 KB) |
| Total      | ~590 lines (~26 KB)   | ~2,800 lines (~99 KB) |

Neither side includes its shared runtime: zngur's `zngur.h` (~356 KB), or
crubit's support crates and headers. The two don't bind the same API either
(crubit's own bindings skip the `Scale` overloads).

Most of zngur's output is the header. For each type, zngur writes a C++ wrapper
class (`rust::cpp::...`), and the specializations of the helpers behind its
Rust-shaped C++ API (`rust::Ref`, `rust::RefMut`, field access, ...), which
C++ code uses to work with Rust values. When only Rust calls the C++ library,
little of that is used. Using the C++ types directly, instead of wrappers,
would shrink it.

## Where the complexity lives

|                    | crubit's own bindings | crubit as a zngur front end |
| ------------------ | --------------------- | --------------------------- |
| C++ to Rust        | `rs_bindings_from_cc/generate_bindings`: ~18k lines of Rust | `generate_zng.rs`: 1,207 lines |
| Rust to C++        | `cc_bindings_from_rs/generate_bindings`: ~18k lines of Rust | `generate_zng.rs`: 790 lines |

The emitters cover a subset of what crubit's generators do, so this compares
where the work moves (zngur does the codegen), not sizes for equal coverage.

## Reproducing the numbers

- The `.zng` file and zngur's output: `bazelisk build
  //examples/zngur/demo:shapes_generated`, then look in
  `bazel-bin/examples/zngur/demo/` (`shapes_generated.zng*`).
- The formatted sizes: format copies of both sides' files with `rustfmt
  --edition 2021` (after removing crubit's `#![rustfmt::skip]`) and
  `clang-format --style=Google`, then count with `wc -lc`.
- crubit's own bindings: crubit's Bazel aspect doesn't load in open-source
  Bazel at this commit. Instead, take the command line of the `ZngFromCc`
  action (`bazelisk aquery 'mnemonic(ZngFromCc,
  //examples/zngur/demo:shapes_generated_zng)'`), and run it from the execution
  root with these changes:
  - drop the `--zng_*` flags;
  - add `--rs_out=...`, `--cc_out=...` and `--namespaces_out=...`;
  - add `"f": ["supported", "experimental"]` to each entry of `--target_args`.
- The generator sizes: `find rs_bindings_from_cc/generate_bindings
  cc_bindings_from_rs/generate_bindings -name '*.rs' ! -name '*test*' | xargs
  cat | wc -l` (minus `generate_zng.rs`).
