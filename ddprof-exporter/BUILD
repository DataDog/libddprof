load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("//third-party:crates.bzl", "all_crate_deps")
package(default_visibility = ["//visibility:public"])

rust_library(
    name = "ddprof-exporter",
    srcs = glob(["src/**/*.rs"]),
    deps = all_crate_deps(),
)

rust_test(
    name = "test",
    srcs = glob(["src/**/*.rs", "tests/**/*.rs"]),
    data = glob(["tests/**/cgroup*", "tests/**/*.pprof"]),
    env = {
        "CARGO_MANIFEST_DIR": "ddprof-exporter",
    
    },
    deps = all_crate_deps(normal = True, normal_dev = True),
)
