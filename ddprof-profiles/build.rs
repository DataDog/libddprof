// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

fn main() -> Result<(), std::io::Error> {
    prost_build::compile_protos(&[ "src/profile.proto"], &["src"])?;
    Ok(())
}