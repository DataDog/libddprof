load("@rules_rust//rust:defs.bzl", "rust_library", "rust_shared_library", "rust_test")

package(default_visibility = ["//visibility:public"])
deps = [
        "//ddprof-exporter",
        "//ddprof-ffi/cargo:chrono",
        "//ddprof-ffi/cargo:libc",
        "//ddprof-ffi/cargo:reqwest",
        "//ddprof-profiles",
    ]

rust_shared_library(
    name = "ddprof-ffi",
    srcs = glob(["src/**/*.rs"]),
    deps = deps,
)

rust_test(
    name = "test",
    srcs = glob(["src/**/*.rs"]),
    deps = deps,
)
