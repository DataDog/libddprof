load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

## rules_rust

http_archive(
    name = "rules_rust",
    sha256 = "dd58513b5c52eadc8c73337315e56908c144fdac94209d032487afbe149586ac",
    strip_prefix = "rules_rust-70f8fb7814d1b2af7f2cb0c4bdfeebf3e2e47ff4",
    urls = [
        # Master branch as of 2022-02-16
        "https://github.com/bazelbuild/rules_rust/archive/70f8fb7814d1b2af7f2cb0c4bdfeebf3e2e47ff4.tar.gz",
    ],
)
load("@rules_rust//rust:repositories.bzl", "rust_repositories")
rust_repositories(version = "1.56.1", edition="2018", rustfmt_version = "1.56.1")

load("//cargo:crates.bzl", "raze_fetch_remote_crates")
raze_fetch_remote_crates()

load("@rules_rust//proto:repositories.bzl", "rust_proto_repositories")
rust_proto_repositories()

load("@rules_rust//proto:transitive_repositories.bzl", "rust_proto_transitive_repositories")
rust_proto_transitive_repositories()

## cargo_raze

http_archive(
    name = "cargo_raze",
    sha256 = "08bfc8859ff686ecb55005a3c4a9cf790115de0abdbcc69cf57b15be0745a859",
    strip_prefix = "cargo-raze-0.14.2",
    url = "https://github.com/google/cargo-raze/archive/v0.14.2.tar.gz",
)

load("@cargo_raze//:repositories.bzl", "cargo_raze_repositories")

cargo_raze_repositories()

load("@cargo_raze//:transitive_deps.bzl", "cargo_raze_transitive_deps")

cargo_raze_transitive_deps()
