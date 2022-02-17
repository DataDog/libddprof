load("@rules_rust//rust:defs.bzl", "rust_library")
load("@rules_rust//cargo:cargo_build_script.bzl", "cargo_build_script")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")

package(default_visibility = ["//visibility:public"])

proto_library(
    name = "proto",
    srcs = ["src/profile.proto"],
)

rust_proto_library(
    name = "profile",
    deps = [":proto"],
)

cargo_build_script(
    name = "ddprof-profiles_build_script",
    srcs = ["build.rs"],
    build_script_env = {
        "PROTOC": "$(execpath @com_google_protobuf//:protoc)",
    },
    data = ["src/profile.proto"],
    tools = ["@com_google_protobuf//:protoc"],
    deps = ["//ddprof-profiles/cargo:prost_build"],
)

rust_library(
    name = "ddprof-profiles",
    srcs = glob(["src/**/*.rs"]),
    deps = [
        ":ddprof-profiles_build_script",
        "//ddprof-profiles/cargo:indexmap",
        "//ddprof-profiles/cargo:prost",
        "//ddprof-profiles/cargo:ux",
    ],
)
