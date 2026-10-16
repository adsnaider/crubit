// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Items that crubit's `.zng` files don't describe yet.

pub fn generic<T>(value: T) -> T {
    value
}

pub struct Wrapper<T>(pub T);

pub struct Borrowed<'a> {
    pub text: &'a str,
}

pub struct HasReference {
    pub value: &'static i32,
    pub count: i32,
}

pub fn maybe(value: i32) -> Option<i32> {
    Some(value)
}

pub fn numbers() -> Vec<i32> {
    vec![1, 2, 3]
}

pub fn slice_len(values: &[u8]) -> usize {
    values.len()
}

pub fn call(f: impl Fn() -> i32) -> i32 {
    f()
}

pub trait Speak {
    fn speak(&self) -> i32;
}

pub mod r#type {
    pub fn inside_keyword_module() -> i32 {
        1
    }
}
