// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

// This lint complains if we implement Hash by hand but derive PartialEq. This
// is a good lint because these two things must agree.
// However, we cannot control the prost-generated code, so cannot remove
// PartialEq or alternatively derive Hash, so we allow this lint.
#![allow(clippy::derive_hash_xor_eq)]

use std::hash::{Hash, Hasher};

include!(concat!(env!("OUT_DIR"), "/pprof.rs"));

impl Copy for Function {}
impl Eq for Function {}

impl Hash for Function {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.name.hash(state);
        self.system_name.hash(state);
        self.filename.hash(state);
        self.start_line.hash(state);
    }
}

impl Copy for Label {}
impl Eq for Label {}

impl Hash for Label {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.str.hash(state);
        self.num.hash(state);
        self.num_unit.hash(state);
    }
}

impl Copy for Line {}
impl Eq for Line {}
impl Hash for Line {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.function_id.hash(state);
        self.line.hash(state);
    }
}

impl Eq for Location {}
impl Hash for Location {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.mapping_id.hash(state);
        self.address.hash(state);
        self.line.hash(state);
        self.is_folded.hash(state);
    }
}

impl Copy for ValueType {}
impl Eq for ValueType {}

impl Hash for ValueType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.r#type.hash(state);
        self.unit.hash(state);
    }
}

#[cfg(test)]
mod test {
    use crate::pprof::{Function, Line, Location, Mapping, Profile, Sample, ValueType};
    use prost::Message;

    #[test]
    fn basic() {
        let mut strings: Vec<::prost::alloc::string::String> = Vec::with_capacity(8);
        strings.push("".into()); // 0
        strings.push("samples".into()); // 1
        strings.push("count".into()); // 2
        strings.push("php".into()); // 3
        strings.push("{main}".into()); // 4
        strings.push("index.php".into()); // 5
        strings.push("test".into()); // 6

        let php_mapping = Mapping {
            id: 1,
            filename: 3,
            ..Default::default()
        };

        let main_function = Function {
            id: 1,
            name: 4,
            system_name: 4,
            filename: 5,
            start_line: 0,
        };

        let test_function = Function {
            id: 2,
            name: 6,
            system_name: 6,
            filename: 5,
            start_line: 3,
        };

        let main_line = Line {
            function_id: main_function.id,
            line: 0,
        };

        let test_line = Line {
            function_id: test_function.id,
            line: 4,
        };

        let main_location = Location {
            id: 1,
            mapping_id: php_mapping.id,
            address: 0,
            line: vec![main_line],
            is_folded: false,
        };

        let test_location = Location {
            id: 2,
            mapping_id: php_mapping.id,
            address: 0,
            line: vec![test_line],
            is_folded: false,
        };

        let profiles = Profile {
            sample_type: vec![ValueType { r#type: 1, unit: 2 }],
            sample: vec![
                Sample {
                    location_id: vec![main_location.id],
                    value: vec![1],
                    label: vec![],
                },
                Sample {
                    location_id: vec![test_location.id, main_location.id],
                    value: vec![1],
                    label: vec![],
                },
            ],
            mapping: vec![php_mapping],
            location: vec![main_location, test_location],
            function: vec![main_function, test_function],
            string_table: strings,
            ..Default::default()
        };

        let mut buffer: Vec<u8> = Vec::new();
        profiles.encode(&mut buffer).expect("encoding to succeed");
        assert!(buffer.len() >= 72);
    }
}
