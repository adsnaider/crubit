// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Free functions and the builtin types.

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn mix(a: f64, b: f32) -> f64 {
    a * f64::from(b)
}

pub fn is_positive(value: i64) -> bool {
    value > 0
}

pub fn low(value: u16) -> u8 {
    (value & 0xff) as u8
}

pub fn count(items: usize) -> isize {
    items as isize
}

pub fn next_char(c: char) -> char {
    char::from_u32(c as u32 + 1).unwrap_or(c)
}

pub fn str_len(s: &str) -> usize {
    s.len()
}

pub fn nothing() {}

pub mod inner {
    pub fn twice(value: i32) -> i32 {
        value * 2
    }
}
