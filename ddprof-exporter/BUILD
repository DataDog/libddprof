load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
package(default_visibility = ["//visibility:public"])

deps = [
        "//ddprof-exporter/cargo:bytes",
        "//ddprof-exporter/cargo:chrono",
        "//ddprof-exporter/cargo:futures",
        "//ddprof-exporter/cargo:http",
        "//ddprof-exporter/cargo:lazy_static",
        "//ddprof-exporter/cargo:libc",
        "//ddprof-exporter/cargo:regex",
        "//ddprof-exporter/cargo:reqwest",
        "//ddprof-exporter/cargo:tokio",
    ]
rust_library(
    name = "ddprof-exporter",
    srcs = glob(["src/**/*.rs"]),
    deps = deps,
)

rust_test(
    name = "test",
    srcs = glob(["src/**/*.rs", "tests/**/*.rs"]),
    data = glob(["tests/**/cgroup*", "tests/**/*.pprof"]),
    env = {
        "CARGO_MANIFEST_DIR": "ddprof-exporter",
    
    },
    deps = deps + ["//ddprof-exporter/cargo:maplit"],
)
