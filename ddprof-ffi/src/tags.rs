use crate::{AsBytes, CharSlice};
use ddprof_exporter::parse_tags;
use ddprof_exporter::tag::Tag;
use std::error::Error;

#[must_use]
#[no_mangle]
pub extern "C" fn ddprof_ffi_Vec_tag_new() -> crate::Vec<Tag> {
    crate::Vec::default()
}

#[no_mangle]
pub extern "C" fn ddprof_ffi_Vec_tag_drop(_: crate::Vec<Tag>) {}

#[repr(C)]
pub enum PushTagResult {
    Ok,
    Err(crate::Vec<u8>),
}

#[no_mangle]
pub extern "C" fn ddprof_ffi_PushTagResult_drop(_: PushTagResult) {}

/// Creates a new Tag from the provided `key` and `value` by doing a utf8
/// lossy conversion, and pushes into the `vec`. The strings `key` and `value`
/// are cloned to avoid FFI lifetime issues.
///
/// # Safety
/// The `vec` must be a valid reference.
/// The CharSlices `key` and `value` must point to at least many bytes as their
/// `.len` properties claim.
#[must_use]
#[no_mangle]
pub unsafe extern "C" fn ddprof_ffi_Vec_tag_push(
    vec: &mut crate::Vec<Tag>,
    key: CharSlice,
    value: CharSlice,
) -> PushTagResult {
    let key = key.to_utf8_lossy().into_owned();
    let value = value.to_utf8_lossy().into_owned();
    match Tag::new(key, value) {
        Ok(tag) => {
            vec.push(tag);
            PushTagResult::Ok
        }
        Err(err) => PushTagResult::Err(err.as_bytes().to_vec().into()),
    }
}

#[repr(C)]
pub struct ParseTagsResult {
    tags: crate::Vec<Tag>,
    error_message: Option<Box<crate::Vec<u8>>>,
}

#[must_use]
#[no_mangle]
pub unsafe extern "C" fn ddprof_ffi_Vec_tag_parse(string: CharSlice) -> ParseTagsResult {
    let string = string.to_utf8_lossy();
    let (tags, error) = parse_tags(string.as_ref());
    ParseTagsResult {
        tags: tags.into(),
        error_message: error.map(|message| Box::new(crate::Vec::from(message.into_bytes()))),
    }
}

#[cfg(test)]
mod tests {
    use crate::tags::*;

    #[test]
    fn empty_tag_name() {
        unsafe {
            let mut tags = ddprof_ffi_Vec_tag_new();
            let result =
                ddprof_ffi_Vec_tag_push(&mut tags, CharSlice::from(""), CharSlice::from("woof"));
            assert!(!matches!(result, PushTagResult::Ok));
        }
    }

    #[test]
    fn test_lifetimes() {
        let mut tags = ddprof_ffi_Vec_tag_new();
        unsafe {
            // make a string here so it has a scoped lifetime
            let key = String::from("key1");
            {
                let value = String::from("value1");
                let result = ddprof_ffi_Vec_tag_push(
                    &mut tags,
                    CharSlice::from(key.as_str()),
                    CharSlice::from(value.as_str()),
                );

                assert!(matches!(result, PushTagResult::Ok));
            }
        }
        let tag = tags.last().unwrap();
        assert_eq!(tag.key(), "key1");
        assert_eq!(tag.value(), "value1");
    }

    #[test]
    fn test_get() {
        unsafe {
            let mut tags = ddprof_ffi_Vec_tag_new();
            let result = ddprof_ffi_Vec_tag_push(
                &mut tags,
                CharSlice::from("sound"),
                CharSlice::from("woof"),
            );
            assert!(matches!(result, PushTagResult::Ok));
            assert_eq!(1, tags.len());
            assert_eq!("sound:woof", tags.get(0).unwrap().to_string());
        }
    }
}
