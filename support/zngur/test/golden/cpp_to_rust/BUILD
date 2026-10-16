"""Golden tests of the `.zng` files of C++ libraries (see zng_golden_test.bzl)."""

load("//support/zngur/test/golden:zng_golden_test.bzl", "cc_zng_golden_test")

package(default_applicable_licenses = ["//:license"])

cc_zng_golden_test(name = "records")

cc_zng_golden_test(name = "non_movable")

cc_zng_golden_test(
    name = "functions",
    cc_deps = ["//support:annotations"],
)

cc_zng_golden_test(name = "unique_ptr")

cc_zng_golden_test(name = "unsupported")

cc_zng_golden_test(
    name = "user_of_records",
    deps = [
        "records",
        "unique_ptr",
    ],
)
