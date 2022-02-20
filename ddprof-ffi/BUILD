load("@rules_rust//rust:defs.bzl", "rust_shared_library", "rust_test")
load("//third-party:crates.bzl", "all_crate_deps")

package(default_visibility = ["//visibility:public"])

rust_shared_library(
    name = "ddprof-ffi",
    srcs = glob(["src/**/*.rs"]),
    deps = [
        "//ddprof-exporter",
        "//ddprof-profiles",
    ] + all_crate_deps(),
)

rust_test(
    name = "test",
    srcs = glob(["src/**/*.rs"]),
    deps = [
        "//ddprof-exporter",
        "//ddprof-profiles",
    ] + all_crate_deps(),
)
