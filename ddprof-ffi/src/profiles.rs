// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

use crate::{Buffer, Slice, Timespec};
use ddprof_profiles as profiles;
use std::convert::{TryFrom, TryInto};
use std::os::raw::c_char;
use std::str::Utf8Error;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ValueType<'a> {
    pub type_: Slice<'a, c_char>,
    pub unit: Slice<'a, c_char>,
}

impl<'a> ValueType<'a> {
    pub fn new(type_: &'a str, unit: &'a str) -> Self {
        Self {
            type_: type_.into(),
            unit: unit.into(),
        }
    }
}

#[repr(C)]
pub struct Period<'a> {
    pub type_: ValueType<'a>,
    pub value: i64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Label<'a> {
    pub key: Slice<'a, c_char>,

    /// At most one of the following must be present
    pub str: Slice<'a, c_char>,
    pub num: i64,

    /// Should only be present when num is present.
    /// Specifies the units of num.
    /// Use arbitrary string (for example, "requests") as a custom count unit.
    /// If no unit is specified, consumer may apply heuristic to deduce the unit.
    /// Consumers may also  interpret units like "bytes" and "kilobytes" as memory
    /// units and units like "seconds" and "nanoseconds" as time units,
    /// and apply appropriate unit conversions to these.
    pub num_unit: Slice<'a, c_char>,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Function<'a> {
    /// Name of the function, in human-readable form if available.
    pub name: Slice<'a, c_char>,

    /// Name of the function, as identified by the system.
    /// For instance, it can be a C++ mangled name.
    pub system_name: Slice<'a, c_char>,

    /// Source file containing the function.
    pub filename: Slice<'a, c_char>,

    /// Line number in source file.
    pub start_line: i64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Line<'a> {
    /// The corresponding profile.Function for this line.
    pub function: Function<'a>,

    /// Line number in source code.
    pub line: i64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Location<'a> {
    /// todo: how to handle unknown mapping?
    pub mapping: Mapping<'a>,

    /// The instruction address for this location, if available.  It
    /// should be within [Mapping.memory_start...Mapping.memory_limit]
    /// for the corresponding mapping. A non-leaf address may be in the
    /// middle of a call instruction. It is up to display tools to find
    /// the beginning of the instruction if necessary.
    pub address: u64,

    /// Multiple line indicates this location has inlined functions,
    /// where the last entry represents the caller into which the
    /// preceding entries were inlined.
    ///
    /// E.g., if memcpy() is inlined into printf:
    ///    line[0].function_name == "memcpy"
    ///    line[1].function_name == "printf"
    pub lines: Slice<'a, Line<'a>>,

    /// Provides an indication that multiple symbols map to this location's
    /// address, for example due to identical code folding by the linker. In that
    /// case the line information above represents one of the multiple
    /// symbols. This field must be recomputed when the symbolization state of the
    /// profile changes.
    pub is_folded: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Mapping<'a> {
    /// Address at which the binary (or DLL) is loaded into memory.
    pub memory_start: u64,

    /// The limit of the address range occupied by this mapping.
    pub memory_limit: u64,

    /// Offset in the binary that corresponds to the first mapped address.
    pub file_offset: u64,

    /// The object this entry is loaded from.  This can be a filename on
    /// disk for the main binary and shared libraries, or virtual
    /// abstractions like "[vdso]".
    pub filename: Slice<'a, c_char>,

    /// A string that uniquely identifies a particular program version
    /// with high probability. E.g., for binaries generated by GNU tools,
    /// it could be the contents of the .note.gnu.build-id field.
    pub build_id: Slice<'a, c_char>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Sample<'a> {
    /// The leaf is at locations[0].
    pub locations: Slice<'a, Location<'a>>,

    /// The type and unit of each value is defined by the corresponding
    /// entry in Profile.sample_type. All samples must have the same
    /// number of values, the same as the length of Profile.sample_type.
    /// When aggregating multiple samples into a single sample, the
    /// result has a list of values that is the element-wise sum of the
    /// lists of the originals.
    pub values: Slice<'a, i64>,

    /// label includes additional context for this sample. It can include
    /// things like a thread id, allocation size, etc
    pub labels: Slice<'a, Label<'a>>,
}

impl<'a> TryFrom<Mapping<'a>> for profiles::api::Mapping<'a> {
    type Error = Utf8Error;

    fn try_from(mapping: Mapping<'a>) -> Result<Self, Self::Error> {
        let filename: &str = mapping.filename.try_into()?;
        let build_id: &str = mapping.build_id.try_into()?;
        Ok(Self {
            memory_start: mapping.memory_start,
            memory_limit: mapping.memory_limit,
            file_offset: mapping.file_offset,
            filename,
            build_id,
        })
    }
}

impl<'a> From<ValueType<'a>> for profiles::api::ValueType<'a> {
    fn from(vt: ValueType<'a>) -> Self {
        Self {
            r#type: vt.type_.try_into().unwrap_or(""),
            unit: vt.unit.try_into().unwrap_or(""),
        }
    }
}

impl<'a> From<&ValueType<'a>> for profiles::api::ValueType<'a> {
    fn from(vt: &ValueType<'a>) -> Self {
        Self {
            r#type: vt.type_.try_into().unwrap_or(""),
            unit: vt.unit.try_into().unwrap_or(""),
        }
    }
}

impl<'a> From<&Period<'a>> for profiles::api::Period<'a> {
    fn from(period: &Period<'a>) -> Self {
        Self {
            r#type: profiles::api::ValueType::from(period.type_),
            value: period.value,
        }
    }
}

impl<'a> TryFrom<Function<'a>> for profiles::api::Function<'a> {
    type Error = Utf8Error;

    fn try_from(function: Function<'a>) -> Result<Self, Self::Error> {
        let name = function.name.try_into()?;
        let system_name = function.system_name.try_into()?;
        let filename = function.filename.try_into()?;
        Ok(Self {
            name,
            system_name,
            filename,
            start_line: function.start_line,
        })
    }
}

impl<'a> TryFrom<Line<'a>> for profiles::api::Line<'a> {
    type Error = Utf8Error;

    fn try_from(line: Line<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            function: line.function.try_into()?,
            line: line.line,
        })
    }
}

impl<'a> TryFrom<Location<'a>> for profiles::api::Location<'a> {
    type Error = Utf8Error;

    fn try_from(location: Location<'a>) -> Result<Self, Self::Error> {
        let mapping: profiles::api::Mapping = location.mapping.try_into()?;
        let mut lines: Vec<profiles::api::Line> = Vec::new();
        for &line in unsafe { location.lines.into_slice() }.iter() {
            lines.push(line.try_into()?);
        }
        Ok(Self {
            mapping,
            address: location.address,
            lines,
            is_folded: location.is_folded,
        })
    }
}

impl<'a> TryFrom<Label<'a>> for profiles::api::Label<'a> {
    type Error = Utf8Error;

    fn try_from(label: Label<'a>) -> Result<Self, Self::Error> {
        let key: &str = label.key.try_into()?;
        let str: Option<&str> = label.str.into();
        let num_unit: Option<&str> = label.num_unit.into();

        Ok(Self {
            key,
            str,
            num: label.num,
            num_unit,
        })
    }
}

impl<'a> TryFrom<Sample<'a>> for profiles::api::Sample<'a> {
    type Error = Utf8Error;

    fn try_from(sample: Sample<'a>) -> Result<Self, Self::Error> {
        let mut locations: Vec<profiles::api::Location> = Vec::with_capacity(sample.locations.len);
        for &location in unsafe { sample.locations.into_slice() }.iter() {
            locations.push(location.try_into()?)
        }

        let values: Vec<i64> = unsafe { sample.values.into_slice() }.to_vec();

        let mut labels: Vec<profiles::api::Label> = Vec::with_capacity(sample.labels.len);
        for &label in unsafe { sample.labels.into_slice() }.iter() {
            labels.push(label.try_into()?);
        }

        Ok(Self {
            locations,
            values,
            labels,
        })
    }
}

/// Create a new profile with the given sample types. Must call
/// `ddprof_ffi_Profile_free` when you are done with the profile.
/// # Safety
/// All slices must be have pointers that are suitably aligned for their type
/// and must have the correct number of elements for the slice.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn ddprof_ffi_Profile_new(
    sample_types: Slice<ValueType>,
    period: Option<&Period>,
) -> Box<ddprof_profiles::Profile> {
    let types: Vec<ddprof_profiles::api::ValueType> =
        sample_types.into_slice().iter().map(Into::into).collect();
    let builder = ddprof_profiles::Profile::builder()
        .sample_types(types)
        .period(period.map(Into::into));

    Box::new(builder.build())
}

#[no_mangle]
/// # Safety
/// The `profile` must point to an object created by another FFI routine in this
/// module, such as `ddprof_ffi_Profile_with_sample_types`.
pub extern "C" fn ddprof_ffi_Profile_free(profile: Box<ddprof_profiles::Profile>) {
    std::mem::drop(profile)
}

#[no_mangle]
/// # Safety
/// The `profile` ptr must point to a valid Profile object created by this
/// module. All pointers inside the `sample` need to be valid for the duration
/// of this call.
/// This call is _NOT_ thread-safe.
pub extern "C" fn ddprof_ffi_Profile_add(
    profile: &mut ddprof_profiles::Profile,
    sample: Sample,
) -> u64 {
    match sample.try_into().map(|s| profile.add(s)) {
        Ok(r) => match r {
            Ok(id) => id.into(),
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

#[repr(C)]
pub struct EncodedProfile {
    start: Timespec,
    end: Timespec,
    buffer: Buffer,
}

impl TryFrom<ddprof_profiles::EncodedProfile> for EncodedProfile {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: ddprof_profiles::EncodedProfile) -> Result<Self, Self::Error> {
        let start = value.start.try_into()?;
        let end = value.end.try_into()?;
        let buffer = Buffer::from_vec(value.buffer);
        Ok(Self { start, end, buffer })
    }
}

/// Destroys the `encoded_profile`
/// # Safety
/// Only safe on profiles created by `ddprof_ffi_Profile_serialize`.
#[no_mangle]
pub unsafe extern "C" fn ddprof_ffi_EncodedProfile_delete(
    encoded_profile: Option<Box<EncodedProfile>>,
) {
    std::mem::drop(encoded_profile)
}

/// Serialize the aggregated profile. Be sure to check the return value and if
/// it's non-null then call `ddprof_ffi_EncodedProfile_delete` on it once
/// done with it.
/// result to free it.
#[no_mangle]
#[must_use]
pub extern "C" fn ddprof_ffi_Profile_serialize(
    profile: &ddprof_profiles::Profile,
) -> Option<Box<EncodedProfile>> {
    match profile.serialize() {
        Ok(encoded_profile) => {
            let ffi = encoded_profile.try_into().ok()?;
            Some(Box::new(ffi))
        }
        Err(e) => {
            eprintln!("Failed to serialize profiles: {}", e);
            None
        }
    }
}

/// Resets all data in `profile` except the sample types and period. Returns
/// true if it successfully reset the profile and false otherwise. The profile
/// remains valid if false is returned.
#[no_mangle]
pub extern "C" fn ddprof_ffi_Profile_reset(profile: &mut ddprof_profiles::Profile) -> bool {
    profile.reset().is_some()
}

#[no_mangle]
/// # Safety
/// Only pass buffers which were created by ddprof routines; do not create one
/// in C and then pass it in. Only call this once per buffer.
pub unsafe extern "C" fn ddprof_ffi_Buffer_free(buffer: Box<Buffer>) {
    std::mem::drop(buffer)
}

#[cfg(test)]
mod test {
    use crate::profiles::*;
    use crate::Slice;

    #[test]
    fn ctor_and_dtor() {
        unsafe {
            let sample_type: *const ValueType = &ValueType::new("samples", "count");
            let profile = ddprof_ffi_Profile_new(Slice::new(sample_type, 1), None);
            ddprof_ffi_Profile_free(profile);
        }
    }

    #[test]
    fn aggregate_samples() {
        unsafe {
            let sample_type: *const ValueType = &ValueType::new("samples", "count");
            let mut profile = ddprof_ffi_Profile_new(Slice::new(sample_type, 1), None);

            let lines = &vec![Line {
                function: Function {
                    name: "{main}".into(),
                    system_name: "{main}".into(),
                    filename: "index.php".into(),
                    start_line: 0,
                },
                line: 0,
            }];

            let mapping = Mapping {
                filename: "php".into(),
                ..Default::default()
            };

            let locations = vec![Location {
                mapping,
                lines: lines.into(),
                ..Default::default()
            }];
            let values: Vec<i64> = vec![1];
            let labels = vec![Label {
                key: Slice::from("pid"),
                num: 101,
                ..Default::default()
            }];

            let sample = Sample {
                locations: Slice::from(&locations),
                values: Slice::from(&values),
                labels: Slice::from(&labels),
            };

            let aggregator = &mut *profile;

            let sample_id1 = ddprof_ffi_Profile_add(aggregator, sample);
            assert_eq!(sample_id1, 1);

            let sample_id2 = ddprof_ffi_Profile_add(aggregator, sample);
            assert_eq!(sample_id1, sample_id2);

            ddprof_ffi_Profile_free(profile);
        }
    }

    unsafe fn provide_distinct_locations_ffi() -> ddprof_profiles::Profile {
        let sample_type: *const ValueType = &ValueType::new("samples", "count");
        let mut profile = ddprof_ffi_Profile_new(Slice::new(sample_type, 1), None);

        let main_lines = vec![Line {
            function: Function {
                name: "{main}".into(),
                system_name: "{main}".into(),
                filename: "index.php".into(),
                start_line: 0,
            },
            line: 0,
        }];

        let test_lines = vec![Line {
            function: Function {
                name: "test".into(),
                system_name: "test".into(),
                filename: "index.php".into(),
                start_line: 3,
            },
            line: 0,
        }];

        let mapping = Mapping {
            filename: "php".into(),
            ..Default::default()
        };

        let main_locations = vec![Location {
            mapping,
            lines: main_lines.as_slice().into(),
            ..Default::default()
        }];
        let test_locations = vec![Location {
            mapping,
            lines: test_lines.as_slice().into(),
            ..Default::default()
        }];
        let values: Vec<i64> = vec![1];
        let labels = vec![Label {
            key: Slice::from("pid"),
            str: Slice::from(""),
            num: 101,
            num_unit: Slice::from(""),
        }];

        let main_sample = Sample {
            locations: Slice::from(main_locations.as_slice()),
            values: Slice::from(values.as_slice()),
            labels: Slice::from(labels.as_slice()),
        };

        let test_sample = Sample {
            locations: Slice::from(test_locations.as_slice()),
            values: Slice::from(values.as_slice()),
            labels: Slice::from(labels.as_slice()),
        };

        let aggregator = &mut *profile;

        let sample_id1 = ddprof_ffi_Profile_add(aggregator, main_sample);
        assert_eq!(sample_id1, 1);

        let sample_id2 = ddprof_ffi_Profile_add(aggregator, test_sample);
        assert_eq!(sample_id2, 2);

        *profile
    }

    #[test]
    fn distinct_locations_ffi() {
        unsafe {
            provide_distinct_locations_ffi();
        }
    }
}
