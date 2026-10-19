"""Golden tests of the `.zng` files of C++ libraries (see zng_golden_test.bzl)."""

load("//support/zngur/test/golden:zng_golden_test.bzl", "cc_zng_golden_test")

package(default_applicable_licenses = ["//:license"])

# With `assume_lifetimes`, crubit gives C++ references default lifetimes, which makes them
# references to C++ objects in the `.zng` files.
_ASSUME_LIFETIMES = ["//features:assume_lifetimes"]

cc_zng_golden_test(
    name = "records",
    aspect_hints = _ASSUME_LIFETIMES,
)

cc_zng_golden_test(
    name = "non_movable",
    aspect_hints = _ASSUME_LIFETIMES,
)

cc_zng_golden_test(
    name = "functions",
    aspect_hints = _ASSUME_LIFETIMES,
    cc_deps = ["//support:annotations"],
)

cc_zng_golden_test(
    name = "unique_ptr",
    aspect_hints = _ASSUME_LIFETIMES,
)

cc_zng_golden_test(
    name = "unsupported",
    aspect_hints = _ASSUME_LIFETIMES,
)

cc_zng_golden_test(
    name = "user_of_records",
    aspect_hints = _ASSUME_LIFETIMES,
    deps = [
        "records",
        "unique_ptr",
    ],
)

# Explicit lifetime annotations (`$(a)`), without `assume_lifetimes`.
cc_zng_golden_test(
    name = "lifetimes",
    cc_deps = ["//support:lifetime_annotations"],
)

# Without lifetime information, C++ references are raw, and functions that take them are `unsafe`.
cc_zng_golden_test(name = "no_lifetimes")
