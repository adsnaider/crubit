// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Calls C++ libraries through the bindings of `cc_zngur_bindings`.

use geo_zngur::geo::{Add, MakeCounter, MakePoint, NextOf, Point};
use shapes_zngur::shapes::{Segment, UnitSegment};
use zngur_lib::{Const, Cpp, CppCtor};

#[test]
fn test_free_function() {
    assert_eq!(Add(2, 3), 5);
}

#[test]
fn test_value_type() {
    let mut p = Point::new_2(3, 4).build();
    assert_eq!(p.Norm2(), 25);
    p.Scale(2);
    // Fields are C++ objects: `&Cpp<i32, Const>` and `&mut Cpp<i32>`.
    let x: &Cpp<i32, Const> = p.x();
    assert_eq!(x.get(), 6);
    p.y_mut().set(1);
    assert_eq!(p.Norm2(), 37);
    let origin = Point::Origin();
    assert_eq!(origin.Norm2(), 0);
}

#[test]
fn test_unique_ptr() {
    let p = MakePoint(1, 2);
    // SAFETY: `p` owns a `Point`, which lives as long as it does.
    let point = unsafe { p.get().as_ref() }.expect("null");
    assert_eq!(point.Norm2(), 5);
}

#[test]
fn test_reference_only_type() {
    let counter = MakeCounter();
    // SAFETY: `counter` owns a `Counter`, which lives as long as it does, and nothing else refers
    // to it meanwhile.
    let counter_ref = unsafe { counter.get().as_mut() }.expect("null");
    assert_eq!(counter_ref.Next(), 1);
    assert_eq!(NextOf(counter_ref), 2);
}

#[test]
fn test_type_of_dependency() {
    let from = Point::new_2(1, 1).build();
    let to = Point::new_2(4, 5).build();
    let segment = Segment::new(&from, &to).build();
    assert_eq!(segment.Length2(), 25);
    assert_eq!(segment.to().Norm2(), 41);
    // The copy constructor is `new_copy`, and takes a `const` object too.
    let to_copy = Point::new_copy(segment.to()).build();
    assert_eq!(to_copy.Norm2(), 41);
    assert_eq!(UnitSegment().Length2(), 1);
}

#[test]
fn test_extra_zng() {
    let segment = UnitSegment();
    assert_eq!(segment.length_squared(), 1);
}
