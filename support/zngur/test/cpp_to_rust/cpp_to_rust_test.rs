// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

//! Calls C++ libraries through the bindings of `cc_zngur_bindings`.

use crubit_zngur_cpp_std::std::unique_ptr;
use geo_zngur::geo::{Add, Counter, MakeCounter, MakePoint, NextOf, Point};
use shapes_zngur::shapes::{Segment, UnitSegment};
use zngur_lib::ZngCppConstruct;

#[test]
fn test_free_function() {
    assert_eq!(Add(2, 3), 5);
}

#[test]
fn test_value_type() {
    let mut p = Point::build((3, 4));
    assert_eq!(Point::Norm2(p.as_ref()), 25);
    Point::Scale(p.as_mut(), 2);
    assert_eq!(Point::x(p.as_ref()).read(), 6);
    Point::y_mut(p.as_mut()).write(1);
    assert_eq!(Point::Norm2(p.as_ref()), 37);
    let origin = Point::Origin();
    assert_eq!(Point::Norm2(origin.as_ref()), 0);
}

#[test]
fn test_unique_ptr() {
    let p = MakePoint(1, 2);
    // SAFETY: `p` owns a `Point`, which lives as long as it does.
    let point = unsafe { unique_ptr::<Point>::get(p.as_ref()).as_cpp_ref() }.expect("null");
    assert_eq!(Point::Norm2(point.as_const()), 5);
}

#[test]
fn test_reference_only_type() {
    let counter = MakeCounter();
    // SAFETY: `counter` owns a `Counter`, which lives as long as it does.
    let counter_ref =
        unsafe { unique_ptr::<Counter>::get(counter.as_ref()).as_cpp_ref() }.expect("null");
    assert_eq!(Counter::Next(counter_ref), 1);
    assert_eq!(NextOf(counter_ref), 2);
}

#[test]
fn test_type_of_dependency() {
    let from = Point::build((1, 1));
    let to = Point::build((4, 5));
    let segment = Segment::build((from.as_ref(), to.as_ref()));
    assert_eq!(Segment::Length2(segment.as_ref()), 25);
    assert_eq!(Point::Norm2(Segment::to(segment.as_ref())), 41);
    assert_eq!(Segment::Length2(UnitSegment().as_ref()), 1);
}

#[test]
fn test_extra_zng() {
    let segment = UnitSegment();
    assert_eq!(Segment::length_squared(segment.as_ref()), 1);
}
