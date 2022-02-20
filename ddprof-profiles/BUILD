load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")
load("@rules_rust//cargo:cargo_build_script.bzl", "cargo_build_script")
load("//third-party:crates.bzl", "all_crate_deps")

package(default_visibility = ["//visibility:public"])

cargo_build_script(
    name = "ddprof-profiles_build_script",
    srcs = ["build.rs"],
    build_script_env = {
        "PROTOC": "$(location @com_google_protobuf//:protoc)",
    },
    data = ["src/profile.proto"],
    tools = ["@com_google_protobuf//:protoc"],
    deps = ["@raze__prost_build__0_8_0//:prost_build"],
)

rust_library(
    name = "ddprof-profiles",
    srcs = glob(["src/**/*.rs"]),
    deps = [":ddprof-profiles_build_script"] + all_crate_deps(),
)

rust_test(
    name = "test",
    srcs = glob(["src/**/*.rs"]),
    deps = [":ddprof-profiles_build_script"] + all_crate_deps(),
)
