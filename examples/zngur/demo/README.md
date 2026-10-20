# Demo: crubit as a zngur front end

A short demo for crubit developers: crubit writes a readable `.zng` file, zngur
generates the bindings from it, and the items that crubit skips can be added
back by hand, without changing crubit or the library.

Run it from the root of the crubit repository:

```sh
examples/zngur/demo/demo.sh             # One step at a time (press Enter).
examples/zngur/demo/demo.sh --no-pause  # All at once.
```

`bazelisk test //examples/zngur/demo/...` runs both programs and the golden tests.

## Files

| File | What it is |
|---|---|
| `shapes.h` | A C++ library: `Point` (held by value), `Polygon` (uses `std::vector<Point>`), `Canvas` (not movable) |
| `generated/shapes.zng` | A copy of the `.zng` file that crubit generates for it (a golden test keeps it up to date) |
| `shapes_extra.zng` | The hand-written additions: `std::vector<Point>`, the two member functions that use it, and `std::make_unique<Canvas>()` |
| `main.rs` | Rust using the library, including the additions |
| `stats.rs` | A Rust crate: `Summary`, `Summary::mean() -> Option<f64>`, and a trait `Shape` |
| `generated/stats.zng` | A copy of the `.zng` file that crubit generates for it |
| `stats_extra.zng` | The hand-written additions: `Option<f64>`, `mean`, the trait and `make_square() -> Box<dyn Shape>` |
| `main.cc` | C++ using the crate, including the additions |
| `comparison.md` | Numbers comparing crubit's own bindings with the `.zng` file, and how to reproduce them |

Each library has two sets of bindings: `*_generated` (only what crubit
generates) and `*_zngur` (the generated file, plus the hand-written one that it
merges).

## Talk track

1. **One BUILD rule.** `cc_zngur_bindings(name = "shapes_generated", lib =
   ":shapes")`. crubit analyzes the headers with Clang, as usual, but only
   writes a `.zng` file.
2. **The `.zng` file is the API.**
   - Fifty lines that read like a spec: layouts, fields, constructors,
     overloads (`Scale`, `Scale_2`, which crubit's own bindings skip), and
     `std::unique_ptr`.
   - What crubit can't describe is listed at the top (`// skipped:`), with the
     reason: here, two member functions that use `std::vector<Point>`.
   - zngur generates all the glue from it, and checks layouts and signatures
     with `static_assert`s on both sides.
3. **Fix it yourself.**
   - `shapes_extra.zng` declares `std::vector<Point>` (its layout and three
     member functions) and redeclares the two skipped member functions.
   - It also adds `std::make_unique<Canvas>()`, so Rust can own a type that it
     can only refer to.
   - The generated file merges it: one line in the BUILD rule (`extra_zng`).
     There's no change to crubit or to the C++ library.
4. **What Rust gets.** The script lists the functions that the merge added:
   `Points`, `Reversed`, `size`, `at`, `push_back` and `make_canvas`.
5. **It runs.** `main.rs` iterates the polygon's points through
   `std::vector<Point>` and draws them on a `Canvas` it created. With crubit's
   `assume_lifetimes` feature, C++ references get lifetimes: member functions
   are `safe` Rust methods (`&'a const self` / `&'a unique var self` in the
   `.zng` file), and a `const Point&` is a `&'a Point<Const>`. References to
   builtins from C++, including field accessors like `p.x()`, are
   `&Cpp<i32, Const>` / `&mut Cpp<i32>`: C++ may change them through other
   paths (they're an `UnsafeCell`), so Rust reads them with `.get()` and
   writes them with `.set()`.
6. **Same in the other direction.** For a Rust crate, crubit skips
   `Option<f64>`, traits and `Box<dyn Shape>`. `stats_extra.zng` adds them, and
   `main.cc` calls `summary.mean()` and `square.area()` from C++.
7. **Why it matters to crubit** (see `comparison.md`):
   - crubit focuses on analysis, and zngur does the codegen: ~2k lines of
     emitters, against ~36k lines of generators (for less coverage, so far).
   - Users unblock themselves, and the `.zng` file shows up in code review
     like an API.
   - Generated bindings mix with zngur's hand-written features: `impl` blocks
     in C++, traits and `Box<dyn …>`.

## Caveats (say them before you're asked)

- **Coverage.** The `.zng` emitters cover less than crubit's own generators.
  crubit's own bindings expose real Rust fields, and in-place construction of
  non-movable types (`Ctor`).
- **Safety.** References follow crubit's lifetime rules, but zngur can't check
  the lifetimes against C++: like crubit's own bindings, they're a promise.
  C++ pointers with lifetime annotations are nullable references
  (`Option<&'a ...>`), and `std::unique_ptr`'s `get()` / `get_mut()` return
  them. Without lifetime information (no `assume_lifetimes` or annotations), C++
  references and pointers are raw pointers, and functions that take them are `unsafe` by
  default (crubit's default). `CRUBIT_UNSAFE_MARK_SAFE` makes them `safe`, and
  the generated file then has `#allow(safe_raw_pointer)`.
- **Generated code size.** zngur generates about 4 to 5 times as much code as
  crubit's own bindings for `shapes.h`, mostly in its C++ header (see
  `comparison.md`). crubit's own part shrinks to the `.zng` file.
- **Hand-written layouts.** A hand-written layout like `std::vector`'s is
  target-specific. zngur checks it at compile time, and `#if cfg!(...)` can
  pick it per target.
- **Not upstream yet.** zngur's C++-to-Rust features are a stack of local
  changes, not upstream.
