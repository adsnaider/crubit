"""Golden tests of the `.zng` files of Rust crates (see zng_golden_test.bzl)."""

load("//support/zngur/test/golden:zng_golden_test.bzl", "rust_zng_golden_test")

package(default_applicable_licenses = ["//:license"])

rust_zng_golden_test(name = "structs")

rust_zng_golden_test(name = "enums")

rust_zng_golden_test(name = "functions")

rust_zng_golden_test(name = "unsupported")

rust_zng_golden_test(
    name = "uses_structs",
    deps = ["structs"],
)
