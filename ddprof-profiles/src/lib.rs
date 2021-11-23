// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

use core::fmt;
use std::borrow::Borrow;
use std::convert::TryInto;
use std::hash::Hash;
use std::ops::AddAssign;
use std::time::{Instant, SystemTime};

use indexmap::{IndexMap, IndexSet};
use prost::{EncodeError, Message};
use ux::u63;

pub mod api;
pub mod pprof;

#[derive(Eq, PartialEq, Hash)]
struct Mapping {
    /// Address at which the binary (or DLL) is loaded into memory.
    pub memory_start: u64,
    /// The limit of the address range occupied by this mapping.
    pub memory_limit: u64,
    /// Offset in the binary that corresponds to the first mapped address.
    pub file_offset: u64,

    /// The object this entry is loaded from.  This can be a filename on
    /// disk for the main binary and shared libraries, or virtual
    /// abstractions like "[vdso]".
    pub filename: PProfId,

    /// A string that uniquely identifies a particular program version
    /// with high probability. E.g., for binaries generated by GNU tools,
    /// it could be the contents of the .note.gnu.build-id field.
    pub build_id: PProfId,
}

#[derive(Eq, PartialEq, Hash)]
struct Function {
    /// Name of the function, in human-readable form if available.
    pub name: PProfId,

    /// Name of the function, as identified by the system.
    /// For instance, it can be a C++ mangled name.
    pub system_name: PProfId,

    /// Source file containing the function.
    pub filename: PProfId,

    /// Line number in source file.
    pub start_line: u63,
}

#[derive(Eq, PartialEq, Hash)]
struct Sample {
    /// The ids recorded here correspond to a Profile.location.id.
    /// The leaf is at location_id[0].
    pub locations: Vec<PProfId>,

    /// label includes additional context for this sample. It can include
    /// things like a thread id, allocation size, etc
    pub labels: Vec<Label>,
}

#[derive(Eq, PartialEq, Hash)]
struct Location {
    /// The id of the corresponding profile.Mapping for this location.
    /// It can be unset if the mapping is unknown or not applicable for
    /// this profile type.
    pub mapping_id: PProfId,

    /// The instruction address for this location, if available.  It
    /// should be within [Mapping.memory_start...Mapping.memory_limit]
    /// for the corresponding mapping. A non-leaf address may be in the
    /// middle of a call instruction. It is up to display tools to find
    /// the beginning of the instruction if necessary.
    pub address: usize,

    /// Multiple line indicates this location has inlined functions,
    /// where the last entry represents the caller into which the
    /// preceding entries were inlined.
    ///
    /// E.g., if memcpy() is inlined into printf:
    ///    line[0].function_name == "memcpy"
    ///    line[1].function_name == "printf"
    pub lines: Vec<Line>,

    /// Provides an indication that multiple symbols map to this location's
    /// address, for example due to identical code folding by the linker. In that
    /// case the line information above represents one of the multiple
    /// symbols. This field must be recomputed when the symbolization state of the
    /// profile changes.
    pub is_folded: bool,
}

#[derive(Eq, PartialEq, Hash)]
struct Line {
    /// The id of the corresponding profile.Function for this line.
    pub function_id: PProfId,

    /// Line number in source code.
    pub line: i64,
}

impl From<&Line> for pprof::Line {
    fn from(line: &Line) -> Self {
        Self {
            function_id: line.function_id.into(),
            line: line.line,
        }
    }
}

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
struct Label {
    /// Index into string table
    pub key: PProfId,

    /// At most one of the following must be present
    ///
    /// Index into string table
    pub str: PProfId,
    pub num: i64,

    /// Should only be present when num is present.
    /// Specifies the units of num.
    /// Use arbitrary string (for example, "requests") as a custom count unit.
    /// If no unit is specified, consumer may apply heuristic to deduce the unit.
    /// Consumers may also  interpret units like "bytes" and "kilobytes" as memory
    /// units and units like "seconds" and "nanoseconds" as time units,
    /// and apply appropriate unit conversions to these.
    ///
    /// Index into string table
    pub num_unit: PProfId,
}

impl From<&Label> for pprof::Label {
    fn from(label: &Label) -> Self {
        Self {
            key: label.key.into(),
            str: label.str.into(),
            num: label.num,
            num_unit: label.num_unit.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct ValueType {
    /// Index into string table.
    pub type_: PProfId,

    /// Index into string table.
    pub unit: PProfId,
}

impl From<&ValueType> for pprof::ValueType {
    fn from(value_type: &ValueType) -> Self {
        Self {
            r#type: value_type.type_.into(),
            unit: value_type.unit.into(),
        }
    }
}

pub struct Profile {
    sample_types: Vec<ValueType>,
    samples: IndexMap<Sample, Vec<i64>>,
    mappings: IndexSet<Mapping>,
    locations: IndexSet<Location>,
    functions: IndexSet<Function>,
    strings: IndexSet<String>,
    started_at: Instant,
    start_time: SystemTime,
    period: i64,
    period_type: Option<ValueType>,
}

pub struct ProfileBuilder<'a> {
    sample_types: Vec<api::ValueType<'a>>,
    period: Option<api::Period<'a>>,
}

impl<'a> ProfileBuilder<'a> {
    pub fn new() -> Self {
        ProfileBuilder {
            sample_types: vec![],
            period: None,
        }
    }

    pub fn sample_types(mut self, mut sample_types: Vec<api::ValueType<'a>>) -> Self {
        std::mem::swap(&mut self.sample_types, &mut sample_types);
        self
    }

    pub fn period(mut self, period: Option<api::Period<'a>>) -> Self {
        self.period = period;
        self
    }

    pub fn build(self) -> Profile {
        let mut profile = Profile::new();
        profile.sample_types = self
            .sample_types
            .iter()
            .map(|vt| ValueType {
                type_: profile.intern(vt.r#type),
                unit: profile.intern(vt.unit),
            })
            .collect();

        if let Some(p) = self.period {
            profile.period = p.value;
            profile.period_type = Some(ValueType {
                type_: profile.intern(p.r#type.r#type),
                unit: profile.intern(p.r#type.unit),
            });
        };

        profile
    }
}

impl<'a> Default for ProfileBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PProfId(usize);

impl From<&PProfId> for u64 {
    fn from(id: &PProfId) -> Self {
        id.0 as u64
    }
}

impl From<PProfId> for u64 {
    fn from(id: PProfId) -> Self {
        id.0.try_into().unwrap_or(0)
    }
}

impl From<&PProfId> for i64 {
    fn from(value: &PProfId) -> Self {
        value.0.try_into().unwrap_or(0)
    }
}

impl From<PProfId> for i64 {
    fn from(value: PProfId) -> Self {
        value.0.try_into().unwrap_or(0)
    }
}

trait DedupExt<T: Eq + Hash> {
    fn dedup(&mut self, item: T) -> usize;

    fn dedup_ref<'a, Q>(&mut self, item: &'a Q) -> usize
    where
        T: Eq + Hash + From<&'a Q> + Borrow<Q>,
        Q: Eq + Hash + ?Sized;
}

impl<T: Sized + Hash + Eq> DedupExt<T> for IndexSet<T> {
    fn dedup(&mut self, item: T) -> usize {
        let (id, _) = self.insert_full(item);
        id
    }

    fn dedup_ref<'a, Q>(&mut self, item: &'a Q) -> usize
    where
        T: Eq + Hash + From<&'a Q> + Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        match self.get_index_of(item) {
            Some(index) => index,
            None => {
                let (index, inserted) = self.insert_full(item.into());
                // This wouldn't make any sense; the item couldn't be found so
                // it was inserted but then it already existed? Screams race-
                // -condition to me!
                assert!(inserted);
                index
            }
        }
    }
}

#[derive(Debug)]
pub struct FullError;

impl fmt::Display for FullError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Full")
    }
}

/// Since the ids are index + 1, we need to take 1 off the size. I also want
/// to restrict the maximum to a 32 bit value; we're gathering way too much
/// data if we ever exceed this in a single profile.
const CONTAINER_MAX: usize = (u32::MAX - 1) as usize;

impl std::error::Error for FullError {}

pub struct EncodedProfile {
    pub start: SystemTime,
    pub end: SystemTime,
    pub buffer: Vec<u8>,
}

impl Profile {
    /// Creates a profile with "now" for the start time.
    /// Initializes the string table to include the empty string.
    /// All other fields are default.
    pub fn new() -> Self {
        /* Do not use Profile's default() impl here or it will cause a stack
         * overflow, since that default impl calls this method.
         */
        let mut profile = Self {
            sample_types: vec![],
            samples: Default::default(),
            mappings: Default::default(),
            locations: Default::default(),
            functions: Default::default(),
            strings: Default::default(),
            started_at: Instant::now(),
            start_time: SystemTime::now(),
            period: 0,
            period_type: None,
        };

        profile.intern("");
        profile
    }

    /// Interns the `str` as a string, returning the id in the string table.
    fn intern(&mut self, str: &str) -> PProfId {
        // strings are special because the empty string is actually allowed at
        // index 0; most other 0's are reserved and cannot exist
        let id = self.strings.dedup_ref(str);
        PProfId(id)
    }

    pub fn builder<'a>() -> ProfileBuilder<'a> {
        ProfileBuilder::new()
    }

    fn add_mapping(&mut self, mapping: &api::Mapping) -> Result<PProfId, FullError> {
        // todo: do full checks as part of intern/dedup
        if self.strings.len() >= CONTAINER_MAX as usize || self.mappings.len() >= CONTAINER_MAX {
            return Err(FullError);
        }

        let filename = self.intern(mapping.filename);
        let build_id = self.intern(mapping.build_id);

        let index = self.mappings.dedup(Mapping {
            memory_start: mapping.memory_start,
            memory_limit: mapping.memory_limit,
            file_offset: mapping.file_offset,
            filename,
            build_id,
        });

        /* PProf reserves mapping 0 for "no mapping", and it won't let you put
         * one in there with all "zero" data either, so we shift the ids.
         */
        Ok(PProfId(index + 1))
    }

    fn add_function(&mut self, function: &api::Function) -> PProfId {
        let name = self.intern(function.name);
        let system_name = self.intern(function.system_name);
        let filename = self.intern(function.filename);

        let index = self.functions.dedup(Function {
            name,
            system_name,
            filename,
            start_line: if function.start_line < 0 {
                u63::new(0)
            } else {
                u63::new(function.start_line as u64)
            },
        });

        /* PProf reserves function 0 for "no function", and it won't let you put
         * one in there with all "zero" data either, so we shift the ids.
         */
        PProfId(index + 1)
    }

    pub fn add(&mut self, sample: api::Sample) -> Result<PProfId, FullError> {
        if sample.values.len() != self.sample_types.len() {
            return Ok(PProfId(0));
        }

        let values = sample.values.iter().copied().collect();
        let labels = sample
            .labels
            .iter()
            .map(|label| {
                let key = self.intern(label.key);
                let str = label.str.map(|s| self.intern(s)).unwrap_or(PProfId(0));
                let num_unit = label.num_unit.map(|s| self.intern(s)).unwrap_or(PProfId(0));

                Label {
                    key,
                    str,
                    num: label.num,
                    num_unit,
                }
            })
            .collect();

        let mut locations: Vec<PProfId> = Vec::with_capacity(sample.locations.len());
        for location in sample.locations.iter() {
            let mapping_id = self.add_mapping(&location.mapping)?;
            let lines: Vec<Line> = location
                .lines
                .iter()
                .map(|line| {
                    let function_id = self.add_function(&line.function);
                    Line {
                        function_id,
                        line: line.line,
                    }
                })
                .collect();

            let index = self.locations.dedup(Location {
                mapping_id,
                address: location.address.try_into().unwrap_or(0),
                lines,
                is_folded: location.is_folded,
            });

            /* PProf reserves location 0. Based on this pattern in other
             * situations, this would be "no location", but I'm not sure how
             * this is logical?
             */
            locations.push(PProfId(index + 1))
        }

        let s = Sample { locations, labels };

        let id = match self.samples.get_index_of(&s) {
            None => {
                self.samples.insert(s, values);
                PProfId(self.samples.len())
            }
            Some(index) => {
                let (_, existing_values) =
                    self.samples.get_index_mut(index).expect("index to exist");
                for (a, b) in existing_values.iter_mut().zip(values) {
                    a.add_assign(b)
                }
                PProfId(index + 1)
            }
        };
        Ok(id)
    }

    fn extract_api_sample_types(&self) -> Option<Vec<api::ValueType>> {
        let mut sample_types: Vec<api::ValueType> = Vec::with_capacity(self.sample_types.len());
        for sample_type in self.sample_types.iter() {
            sample_types.push(api::ValueType {
                r#type: self.strings.get_index(sample_type.type_.0)?.as_str(),
                unit: self.strings.get_index(sample_type.unit.0)?.as_str(),
            })
        }
        Some(sample_types)
    }

    /// Resets all data except the sample types and period. Returns the
    /// previous Profile on success.
    pub fn reset(&mut self) -> Option<Profile> {
        /* We have to map over the types because the order of the strings is
         * not generally guaranteed, so we can't just copy the underlying
         * structures.
         */
        let sample_types: Vec<api::ValueType> = self.extract_api_sample_types()?;

        let mut profile = ProfileBuilder::new()
            .sample_types(sample_types)
            .period(match &self.period_type {
                Some(t) => Some(api::Period {
                    r#type: api::ValueType {
                        r#type: self.strings.get_index(t.type_.0)?.as_str(),
                        unit: self.strings.get_index(t.unit.0)?.as_str(),
                    },
                    value: self.period,
                }),
                None => None,
            })
            .build();

        std::mem::swap(&mut *self, &mut profile);
        Some(profile)
    }

    /// Serialize the aggregated profile.
    pub fn serialize(&self) -> Result<EncodedProfile, EncodeError> {
        let profile: pprof::Profile = self.into();
        let mut buffer: Vec<u8> = Vec::new();
        profile.encode(&mut buffer)?;
        Ok(EncodedProfile {
            start: self.start_time,
            end: SystemTime::now(),
            buffer,
        })
    }

    pub fn get_string(&self, id: PProfId) -> Option<&String> {
        self.strings.get_index(id.0)
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&Profile> for pprof::Profile {
    fn from(profile: &Profile) -> Self {
        pprof::Profile {
            sample_type: profile.sample_types.iter().map(Into::into).collect(),
            sample: profile
                .samples
                .iter()
                .map(|(sample, values)| pprof::Sample {
                    location_id: sample.locations.iter().map(Into::into).collect(),
                    value: values.to_vec(),
                    label: sample.labels.iter().map(Into::into).collect(),
                })
                .collect(),
            mapping: profile
                .mappings
                .iter()
                .enumerate()
                .map(|(index, mapping)| pprof::Mapping {
                    id: (index + 1) as u64,
                    memory_start: mapping.memory_start,
                    memory_limit: mapping.memory_limit,
                    file_offset: mapping.file_offset,
                    filename: mapping.filename.into(),
                    build_id: mapping.build_id.into(),
                    ..Default::default() // todo: support detailed Mapping info
                })
                .collect(),
            location: profile
                .locations
                .iter()
                .enumerate()
                .map(|(index, location)| pprof::Location {
                    id: (index + 1) as u64,
                    mapping_id: location.mapping_id.into(),
                    address: location.address as u64,
                    line: location.lines.iter().map(Into::into).collect(),
                    is_folded: location.is_folded,
                })
                .collect(),
            function: profile
                .functions
                .iter()
                .enumerate()
                .map(|(index, function)| {
                    let start_line: u64 = function.start_line.into();
                    pprof::Function {
                        id: (index + 1) as u64,
                        name: function.name.into(),
                        system_name: function.system_name.into(),
                        filename: function.filename.into(),
                        start_line: start_line.try_into().unwrap_or(0),
                    }
                })
                .collect(),
            string_table: profile.strings.iter().map(Into::into).collect(),
            time_nanos: profile
                .start_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as i64),
            duration_nanos: profile
                .started_at
                .elapsed()
                .as_nanos()
                .try_into()
                .unwrap_or(0),
            period: profile.period,
            period_type: profile.period_type.as_ref().map(Into::into),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod api_test {
    use crate::{api, pprof, PProfId, Profile};

    #[test]
    fn interning() {
        let sample_types = vec![api::ValueType {
            r#type: "samples",
            unit: "count",
        }];
        let mut profiles = Profile::builder().sample_types(sample_types).build();

        /* There have been 3 strings: "", "samples", and "count". Since the interning index starts at
         * zero, this means the next string will be 3.
         */
        const EXPECTED_ID: PProfId = PProfId(3);

        let string = "a";
        let id1 = profiles.intern(string);
        let id2 = profiles.intern(string);

        assert_eq!(id1, id2);
        assert_eq!(id1, EXPECTED_ID);
    }

    #[test]
    fn api() {
        let sample_types = vec![
            api::ValueType {
                r#type: "samples",
                unit: "count",
            },
            api::ValueType {
                r#type: "wall-time",
                unit: "nanoseconds",
            },
        ];

        let mapping = api::Mapping {
            filename: "php",
            ..Default::default()
        };

        let index = api::Function {
            filename: "index.php",
            ..Default::default()
        };

        let locations = vec![
            api::Location {
                mapping,
                lines: vec![api::Line {
                    function: api::Function {
                        name: "phpinfo",
                        system_name: "phpinfo",
                        filename: "index.php",
                        start_line: 0,
                    },
                    line: 0,
                }],
                ..Default::default()
            },
            api::Location {
                mapping,
                lines: vec![api::Line {
                    function: index,
                    line: 3,
                }],
                ..Default::default()
            },
        ];

        let mut profile = Profile::builder().sample_types(sample_types).build();
        let sample_id = profile
            .add(api::Sample {
                locations,
                values: vec![1, 10000],
                labels: vec![],
            })
            .expect("add to succeed");

        assert_eq!(sample_id, PProfId(1));
    }

    fn provide_distinct_locations() -> crate::Profile {
        let sample_types = vec![api::ValueType {
            r#type: "samples",
            unit: "count",
        }];

        let main_lines = vec![api::Line {
            function: api::Function {
                name: "{main}",
                system_name: "{main}",
                filename: "index.php",
                start_line: 0,
            },
            line: 0,
        }];

        let test_lines = vec![api::Line {
            function: api::Function {
                name: "test",
                system_name: "test",
                filename: "index.php",
                start_line: 3,
            },
            line: 0,
        }];

        let mapping = api::Mapping {
            filename: "php",
            ..Default::default()
        };

        let main_locations = vec![api::Location {
            mapping,
            lines: main_lines,
            ..Default::default()
        }];
        let test_locations = vec![api::Location {
            mapping,
            lines: test_lines,
            ..Default::default()
        }];
        let values: Vec<i64> = vec![1];
        let labels = vec![api::Label {
            key: "pid",
            num: 101,
            ..Default::default()
        }];

        let main_sample = api::Sample {
            locations: main_locations,
            values: values.clone(),
            labels: labels.clone(),
        };

        let test_sample = api::Sample {
            locations: test_locations,
            values,
            labels,
        };

        let mut profile = Profile::builder().sample_types(sample_types).build();

        let sample_id1 = profile.add(main_sample).expect("profile to not be full");
        assert_eq!(sample_id1, PProfId(1));

        let sample_id2 = profile.add(test_sample).expect("profile to not be full");
        assert_eq!(sample_id2, PProfId(2));

        profile
    }

    #[test]
    fn impl_from_profile_for_pprof_profile() {
        let profile: pprof::Profile = (&provide_distinct_locations()).into();

        assert_eq!(profile.sample.len(), 2);
        assert_eq!(profile.mapping.len(), 1);
        assert_eq!(profile.location.len(), 2);
        assert_eq!(profile.function.len(), 2);

        for (index, mapping) in profile.mapping.iter().enumerate() {
            assert_eq!((index + 1) as u64, mapping.id);
        }

        for (index, location) in profile.location.iter().enumerate() {
            assert_eq!((index + 1) as u64, location.id);
        }

        for (index, function) in profile.function.iter().enumerate() {
            assert_eq!((index + 1) as u64, function.id);
        }

        let sample = profile.sample.get(0).expect("index 0 to exist");
        assert_eq!(sample.label.len(), 1);
        let label = sample.label.get(0).expect("index 0 to exist");
        let key = profile
            .string_table
            .get(label.key as usize)
            .expect("index to exist");
        let str = profile
            .string_table
            .get(label.str as usize)
            .expect("index to exist");
        let num_unit = profile
            .string_table
            .get(label.num_unit as usize)
            .expect("index to exist");
        assert_eq!(key, "pid");
        assert_eq!(label.num, 101);
        assert_eq!(str, "");
        assert_eq!(num_unit, "");
    }

    #[test]
    fn reset() {
        let mut profile = provide_distinct_locations();
        /* This set of asserts is to make sure it's a non-empty profile that we
         * are working with so that we can test that reset works.
         */
        assert!(!profile.functions.is_empty());
        assert!(!profile.locations.is_empty());
        assert!(!profile.mappings.is_empty());
        assert!(!profile.samples.is_empty());
        assert!(!profile.sample_types.is_empty());
        assert!(profile.period_type.is_none());

        let prev = profile.reset().expect("reset to succeed");

        // These should all be empty now
        assert!(profile.functions.is_empty());
        assert!(profile.locations.is_empty());
        assert!(profile.mappings.is_empty());
        assert!(profile.samples.is_empty());

        assert_eq!(profile.period, prev.period);
        assert_eq!(profile.period_type, prev.period_type);
        assert_eq!(profile.sample_types, prev.sample_types);

        // The string table should have at least the empty string:
        assert!(!profile.strings.is_empty());
        // The empty string should be at position 0
        assert_eq!(
            profile.get_string(PProfId(0)).expect("index 0 to be found"),
            ""
        );

        // The start time should be newer after reset, as Instant is monotonic.
        assert!(profile.started_at >= prev.started_at);
    }

    #[test]
    fn reset_period() {
        /* The previous test (reset) checked quite a few properties already, so
         * this one will focus only on the period.
         */
        let mut profile = provide_distinct_locations();

        let period: i64 = 10000;
        profile.period_type = Some(crate::ValueType {
            type_: profile.intern("wall-time"),
            unit: profile.intern("nanoseconds"),
        });
        profile.period = period;

        let prev = profile.reset().expect("reset to succeed");
        assert_eq!(profile.period, prev.period);
        assert_eq!(profile.period, period);

        /* The strings may not be interned to the same location, but they should
         * still match if we resolve them.
         */
        let period_type = profile.period_type.expect("period_type to exist");
        let r#type = period_type.type_;
        let unit = period_type.unit;
        assert_eq!(
            profile.get_string(r#type).expect("string to be found"),
            "wall-time"
        );
        assert_eq!(
            profile.get_string(unit).expect("string to be found"),
            "nanoseconds"
        );
    }
}
