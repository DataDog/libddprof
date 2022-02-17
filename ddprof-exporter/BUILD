load("@rules_rust//rust:defs.bzl", "rust_library")
package(default_visibility = ["//visibility:public"])

rust_library(
    name = "ddprof-exporter",
    srcs = glob(["src/**/*.rs"]),
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
    ],
)
