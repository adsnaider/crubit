// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#![feature(rustc_private)]

// Links rustc's shared library (once), which the emitter's rustc crates need.
extern crate rustc_driver;

use generate_zng::{generate_zng, ZngImport, ZngOptions};
use run_compiler_test_support::run_compiler_for_testing;
use test_helpers::bindings_db_for_tests;

/// Returns the `.zng` file generated for the given Rust `source`.
fn zng_for(source: &str) -> String {
    zng_with_options(source, ZngOptions::default())
}

fn zng_with_options(source: &str, options: ZngOptions) -> String {
    run_compiler_for_testing(source, |tcx| {
        generate_zng(&bindings_db_for_tests(tcx), &options)
            .expect("generating the .zng file failed")
    })
}

#[test]
fn test_free_functions() {
    let zng = zng_for(
        r#"
        pub fn add(x: i32, y: i32) -> i32 { x + y }
        pub mod nested {
            pub fn noop() {}
        }
        pub fn sub(x: i32, y: i32) -> i32 { x - y }
        "#,
    );
    // Functions are grouped by module, regardless of the order they're declared in.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out

mod ::rust_out {
    fn add(i32, i32) -> i32;
    fn sub(i32, i32) -> i32;
}
mod ::rust_out::nested {
    fn noop();
}
"#
    );
}

#[test]
fn test_structs() {
    let zng = zng_for(
        r#"
        #[derive(Clone, Copy)]
        #[repr(C)]
        pub struct Point {
            pub x: i32,
            pub y: i32,
        }

        impl Point {
            pub fn new(x: i32, y: i32) -> Self { Self { x, y } }
            pub fn x(&self) -> i32 { self.x }
            pub fn set_x(&mut self, x: i32) { self.x = x; }
            pub fn is_origin(self) -> bool { self.x == 0 && self.y == 0 }
        }

        #[repr(C)]
        pub struct Opaque {
            pub tag: char,
            secret: u64,
        }

        impl Drop for Opaque {
            fn drop(&mut self) {}
        }

        pub fn get_point(o: &Opaque) -> Point { Point::new(o.secret as i32, 0) }
        "#,
    );
    // `bool` and `char` must be declared; `Opaque` isn't `Copy`, so zngur gives it a destructor;
    // the private field `secret` is omitted; offsets and layouts are exact; the method `x` is `x_`
    // in C++, where it can't have the name of a field.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out

type bool {
    #layout(size = 1, align = 1);
    wellknown_traits(Copy);
}

type char {
    #layout(size = 4, align = 4);
    wellknown_traits(Copy);
}

type ::rust_out::Opaque {
    #layout(size = 16, align = 8);

    field tag (offset = 0, type = char);
}

type ::rust_out::Point {
    #layout(size = 8, align = 4);
    wellknown_traits(Copy);

    field x (offset = 0, type = i32);
    field y (offset = 4, type = i32);

    fn new(i32, i32) -> ::rust_out::Point as new_;
    fn x<'a>(&'a self) -> i32 as x_;
    fn set_x<'a>(&'a mut self, i32);
    fn is_origin(self) -> bool;
}

mod ::rust_out {
    fn get_point<'a>(&'a ::rust_out::Opaque) -> ::rust_out::Point;
}
"#
    );
}

#[test]
fn test_traits() {
    let zng = zng_for(
        r#"
        #[derive(Clone, Debug, Default)]
        pub struct Counter {
            pub count: u64,
        }

        #[derive(Clone, Copy, Debug)]
        pub struct Id(pub u32);

        #[derive(Clone)]
        pub struct Named(pub u8);
        impl Named {
            pub fn clone(&self) -> u8 { self.0 }
        }
        "#,
    );
    // `Debug` is a zngur well-known trait; `clone` (for non-`Copy` types) and `default` call the
    // trait implementations, unless there's an inherent method with the same name.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out

type ::rust_out::Counter {
    #layout(size = 8, align = 8);
    wellknown_traits(Debug);

    field count (offset = 0, type = u64);

    fn clone(&self) -> ::rust_out::Counter use ::core::clone::Clone;
    fn default() -> ::rust_out::Counter use ::core::default::Default;
}

type ::rust_out::Id {
    #layout(size = 4, align = 4);
    wellknown_traits(Copy, Debug);

    field 0 (offset = 0, type = u32);
}

type ::rust_out::Named {
    #layout(size = 1, align = 1);

    field 0 (offset = 0, type = u8);

    fn clone<'a>(&'a self) -> u8;
}
"#
    );
}

#[test]
fn test_str() {
    let zng = zng_for(
        r#"
        pub fn greeting_len(name: &str) -> usize { name.len() }
        "#,
    );
    // `str` must be declared, with the methods that C++ needs to read it.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out

type str {
    wellknown_traits(?Sized);

    fn as_ptr(&self) -> *const u8;
    fn len(&self) -> usize;
}

mod ::rust_out {
    fn greeting_len<'a>(&'a str) -> usize;
}
"#
    );
}

#[test]
fn test_function_lifetimes() {
    let zng = zng_for(
        r#"
        pub fn longest<'x>(a: &'x str, b: &'x str) -> &'x str { if a.len() > b.len() { a } else { b } }
        pub fn first<'x>(a: &'x str, _b: &str) -> &'x str { a }
        pub fn name() -> &'static str { "name" }
        "#,
    );
    // Named lifetimes keep their names, elided ones get fresh names, and `'static` needs no
    // declaration.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out

type str {
    wellknown_traits(?Sized);

    fn as_ptr(&self) -> *const u8;
    fn len(&self) -> usize;
}

mod ::rust_out {
    fn first<'x, 'a>(&'x str, &'a str) -> &'x str;
    fn longest<'x>(&'x str, &'x str) -> &'x str;
    fn name() -> &'static str;
}
"#
    );
}

#[test]
fn test_skipped_items() {
    let zng = zng_for(
        r#"
        pub struct Generic<T>(pub T);
        pub union Union { pub x: i32 }
        pub fn takes_isize(x: isize) -> isize { x }
        pub unsafe fn unsafe_fn() {}
        pub const CONSTANT: i32 = 42;
        pub fn uses_string(s: String) -> String { s }
        pub fn generic<T>(t: T) -> T { t }
        "#,
    );
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out
//
// Items that are not (yet) supported in this file:
// skipped: ::rust_out::CONSTANT: constant items are not supported yet
// skipped: ::rust_out::Generic: generic types are not supported yet
// skipped: ::rust_out::Union: union items are not supported yet
// skipped: ::rust_out::generic: generic functions are not supported yet
// skipped: ::rust_out::takes_isize: `isize` is not supported by zngur
// skipped: ::rust_out::unsafe_fn: unsafe functions are not supported yet
// skipped: ::rust_out::uses_string: type `std::string::String` is from `alloc`, whose `.zng` file isn't imported
"#
    );
}

#[test]
fn test_cpp_reserved_names() {
    let zng = zng_for(
        r#"
        pub struct Keywords { pub int: i32 }
        impl Keywords {
            pub fn operator(&self) -> i32 { self.int }
        }
        pub fn explicit() {}
        pub mod new {
            pub fn delete() {}
        }
        "#,
    );
    // Methods get crubit's C++ name via `as`; zngur can't rename anything else, so it's skipped.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out
//
// Items that are not (yet) supported in this file:
// skipped: ::rust_out::Keywords::int: field: the name is reserved in C++, and zngur can't rename fields
// skipped: ::rust_out::explicit: `explicit` is reserved in C++, and zngur can't rename it
// skipped: ::rust_out::new::delete: `new` is reserved in C++, and zngur can't rename it

type ::rust_out::Keywords {
    #layout(size = 4, align = 4);

    fn operator<'a>(&'a self) -> i32 as operator_;
}
"#
    );
}

#[test]
fn test_imports_and_merges() {
    let dir = tempfile::tempdir().unwrap();
    let core_zng = dir.path().join("core.zng");
    std::fs::write(
        &core_zng,
        r#"
type bool {
    #layout(size = 1, align = 1);
    wellknown_traits(Copy);
}

type ::core::time::Duration {
    #layout(size = 16, align = 8);
    wellknown_traits(Copy);
}
"#,
    )
    .unwrap();
    let options = ZngOptions {
        imports: vec![ZngImport {
            crate_name: "core".to_string(),
            zng_file: core_zng,
            import_path: "../core/core.zng".to_string(),
        }],
        merges: vec!["./extra.zng".to_string()],
    };
    let zng = zng_with_options(
        r#"
        pub fn is_long(d: std::time::Duration) -> bool { d.as_secs() > 60 }
        pub fn ordering(_o: std::cmp::Ordering) {}
        "#,
        options,
    );
    // Types of imported files can be used (and builtins they declare aren't declared again);
    // other types of the same crate can't.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out
//
// Items that are not (yet) supported in this file:
// skipped: ::rust_out::ordering: type `std::cmp::Ordering` isn't in the `.zng` file of `core`

import "../core/core.zng";

mod ::rust_out {
    fn is_long(::core::time::Duration) -> bool;
}

// Hand-written additions.
merge "./extra.zng";
"#
    );
}

#[test]
fn test_names_and_lifetimes() {
    let zng = zng_for(
        r#"
        pub mod r#move {
            pub fn noop() {}
        }

        pub struct Borrowed<'a> {
            pub name: &'a str,
        }

        pub struct Static {
            pub name: &'static str,
            pub len: usize,
        }

        pub struct Conflicting {
            pub len: usize,
        }
        impl Conflicting {
            pub fn len(&self) -> usize { self.len }
            pub fn len_(&self) -> usize { self.len }
        }
        "#,
    );
    // zngur can't spell raw identifiers or lifetimes, and in C++ a member function can't have
    // the name of a field.
    assert_eq!(
        zng,
        r#"// Automatically @generated zngur IDL for the following Rust crate:
// rust_out
//
// Items that are not (yet) supported in this file:
// skipped: ::rust_out::Borrowed: types with lifetime parameters are not supported yet
// skipped: ::rust_out::Static::name: field: fields of reference type are not supported yet
// skipped: ::rust_out::move::noop: `move` is a Rust keyword, which zngur can't spell

type ::rust_out::Conflicting {
    #layout(size = 8, align = 8);

    field len (offset = 0, type = usize);

    fn len<'a>(&'a self) -> usize as len__;
    fn len_<'a>(&'a self) -> usize;
}

type ::rust_out::Static {
    #layout(size = 24, align = 8);

    field len (offset = 16, type = usize);
}
"#
    );
}
