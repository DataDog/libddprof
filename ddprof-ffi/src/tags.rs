use crate::CharSlice;
use ddprof_exporter::tag::Tag;
use std::borrow::Cow;
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
    let key = String::from_utf8_lossy(key.as_bytes());
    let value = String::from_utf8_lossy(value.as_bytes());
    match Tag::new(key, value) {
        Ok(tag) => {
            vec.push(tag);
            PushTagResult::Ok
        }
        Err(err) => PushTagResult::Err(err.as_bytes().to_vec().into()),
    }
}

fn parse_tag_chunk<'a>(chunk: &'a str) -> Result<Tag, Cow<'static, str>> {
    if let Some(first_colon_position) = chunk.find(':') {
        if first_colon_position == 0 {
            return Err(format!("tag cannot start with a colon: \"{}\"", chunk).into());
        }
        let name = &chunk[..first_colon_position];
        let value = &chunk[(first_colon_position + 1)..];
        Tag::new(Cow::Owned(name.into()), Cow::Owned(value.into()))
    } else {
        Tag::new(Cow::Owned(chunk.into()), Cow::Borrowed(""))
    }
}

/// Parse a string of tags typically provided by environment variables
/// The tags are expected to be either space or comma separated:
///     "key1:value1,key2:value2"
///     "key1:value1 key2:value2"
/// Tag names and values are required and may not be empty.
///
/// Returns a tuple of the correctly parsed tags and an optional error message
/// describing issues encountered during parsing.
fn parse_tags(str: &str) -> (Vec<Tag>, Option<String>) {
    let chunks = str
        .split(&[',', ' '][..])
        .filter(|str| !str.is_empty())
        .map(parse_tag_chunk);

    let mut tags = vec![];
    let mut error_message = String::new();
    for result in chunks {
        match result {
            Ok(tag) => tags.push(tag),
            Err(err) => {
                if error_message.is_empty() {
                    error_message += "Errors while parsing tags: ";
                } else {
                    error_message += ", ";
                }
                error_message += err.as_ref();
            }
        }
    }

    (
        tags,
        if error_message.is_empty() {
            None
        } else {
            Some(error_message)
        },
    )
}

#[repr(C)]
pub struct ParseTagsResult {
    tags: crate::Vec<Tag>,
    error_message: Option<Box<crate::Vec<u8>>>,
}

#[must_use]
#[no_mangle]
pub extern "C" fn ddprof_ffi_Vec_tag_parse(string: CharSlice) -> ParseTagsResult {
    match unsafe { string.try_to_utf8() } {
        Ok(string) => {
            let (tags, error) = parse_tags(string);
            ParseTagsResult {
                tags: tags.into(),
                error_message: error
                    .map(|message| Box::new(crate::Vec::from(message.into_bytes()))),
            }
        }
        Err(err) => ParseTagsResult {
            tags: crate::Vec::default(),
            error_message: Some(Box::new(crate::Vec::from(&err as &dyn Error))),
        },
    }
}

#[must_use]
#[allow(clippy::ptr_arg)]
#[no_mangle]
pub extern "C" fn ddprof_ffi_Vec_tag_clone(tags: &crate::Vec<Tag>) -> crate::Vec<Tag> {
    tags.iter()
        .map(|tag| tag.clone().into_owned())
        .collect::<Vec<Tag>>()
        .into()
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
    fn test_dup() {
        unsafe {
            let mut tags = ddprof_ffi_Vec_tag_new();
            let result = ddprof_ffi_Vec_tag_push(
                &mut tags,
                CharSlice::from("sound"),
                CharSlice::from("woof"),
            );
            assert!(matches!(result, PushTagResult::Ok));

            let mut cloned_tags = ddprof_ffi_Vec_tag_clone(&tags);
            let result = ddprof_ffi_Vec_tag_push(&mut cloned_tags, "host".into(), "dog".into());
            assert!(matches!(result, PushTagResult::Ok));
        }
    }

    #[test]
    fn test_parse_tags() {
        // See the docs for what we convey to users about tags:
        // https://docs.datadoghq.com/getting_started/tagging/

        let cases = [
            ("", vec![]),
            (",", vec![]),
            (" , ", vec![]),
            (
                "env:staging:east",
                vec![Tag::new("env".into(), "staging:east".into()).unwrap()],
            ),
            ("value", vec![Tag::new("value".into(), "".into()).unwrap()]),
            (
                "state:utah,state:idaho",
                vec![
                    Tag::new("state".into(), "utah".into()).unwrap(),
                    Tag::new("state".into(), "idaho".into()).unwrap(),
                ],
            ),
            (
                "key1:value1 key2:value2 key3:value3",
                vec![
                    Tag::new("key1".into(), "value1".into()).unwrap(),
                    Tag::new("key2".into(), "value2".into()).unwrap(),
                    Tag::new("key3".into(), "value3".into()).unwrap(),
                ],
            ),
            (
                // Testing consecutive separators being collapsed
                "key1:value1, key2:value2 ,key3:value3 , key4:value4",
                vec![
                    Tag::new("key1".into(), "value1".into()).unwrap(),
                    Tag::new("key2".into(), "value2".into()).unwrap(),
                    Tag::new("key3".into(), "value3".into()).unwrap(),
                    Tag::new("key4".into(), "value4".into()).unwrap(),
                ],
            ),
            ("key1:", vec![Tag::new("key1".into(), "".into()).unwrap()]),
        ];

        for case in cases {
            let expected = case.1;
            let (actual, error_message) = parse_tags(case.0);
            assert_eq!(expected, actual);
            assert!(error_message.is_none());
        }
    }
}
