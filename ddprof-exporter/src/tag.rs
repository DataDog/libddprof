use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Eq, PartialEq)]
pub struct Tag {
    key: Cow<'static, str>,
    value: Cow<'static, str>,
}

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tag")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish()
    }
}

// Any type which implements Display automatically has to_string.
impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // A tag isn't supposed to end with a colon, so if there isn't a value
        // then don't follow the tag with a colon.
        if self.value.is_empty() {
            write!(f, "{}", self.key)
        } else {
            write!(f, "{}:{}", self.key, self.value)
        }
    }
}

impl Tag {
    pub fn new<IntoCow: Into<Cow<'static, str>>>(
        key: IntoCow,
        value: IntoCow,
    ) -> Result<Self, Cow<'static, str>> {
        let key = key.into();
        let value = value.into();
        if key.is_empty() {
            return Err("tag key was empty".into());
        }

        let first_valid_char = key
            .chars()
            .find(|char| *char != std::char::REPLACEMENT_CHARACTER && !char.is_whitespace());

        match first_valid_char {
            None => Err("tag contained only whitespace or invalid unicode characters".into()),
            Some(':') => Err("tag cannot start with a colon".into()),
            Some(_) => Ok(Self { key, value }),
        }
    }

    pub fn key(&self) -> &Cow<str> {
        &self.key
    }
    pub fn value(&self) -> &Cow<str> {
        &self.value
    }

    pub fn into_owned(mut self) -> Self {
        self.key = self.key.to_owned();
        self.value = self.value.to_owned();
        self
    }
}

pub fn parse_tag_chunk<'a>(chunk: &'a str) -> Result<Tag, Cow<'static, str>> {
    if let Some(first_colon_position) = chunk.find(':') {
        // A tag which leads with a colon isn't explicitly invalid, but what
        // are we supposed to do about it?
        if first_colon_position == 0 {
            return Err("tag cannot start with a colon".into());
        }
        if let Some(last_char) = chunk.chars().last() {
            if last_char == ':' {
                return Err("tag cannot end with a colon".into());
            }
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
pub fn parse_tags(str: &str) -> (Vec<Tag>, Option<String>) {
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

    let error_message = if error_message.is_empty() {
        None
    } else {
        Some(error_message)
    };
    (tags, error_message)
}

#[cfg(test)]
mod tests {
    use crate::{parse_tag_chunk, parse_tags, Tag};

    #[test]
    fn test_empty_key() {
        let _ = Tag::new("", "woof").expect_err("empty key is not allowed");
    }

    #[test]
    fn test_empty_value() {
        let tag = Tag::new("key1", "").expect("empty value is okay");
        assert_eq!("key1", tag.to_string()); // notice no trailing colon!
    }

    #[test]
    fn test_bad_utf8() {
        // 0b1111_0xxx is the start of a 4-byte sequence, but there aren't any
        // more chars, so it  will get converted into the utf8 replacement
        // character. This results in a string with a space (32) and a
        // replacement char, so it should be an error (no valid chars).
        let bytes = &[32, 0b1111_0111];
        let key = String::from_utf8_lossy(bytes);
        let _ = Tag::new(key, "value".into()).expect_err("invalid tag is rejected");
    }

    #[test]
    fn test_value_has_colon() {
        let result = Tag::new("env", "staging:east").expect("values can have colons");
        assert_eq!("env:staging:east", result.to_string());
    }

    #[test]
    fn test_suspicious_tags() {
        // Based on tag rules, these should all fail. However, there is a risk
        // that profile tags will then differ or cause failures compared to
        // trace tags. These require cross-team, cross-language collaboration.
        let cases = [
            ("key".to_string(), "value-ends-with-colon:".to_owned()),
            (
                "the-tag-length-is-over-200-characters".repeat(6),
                "value".to_owned(),
            ),
        ];

        for case in cases {
            let result = Tag::new(case.0, case.1);
            // Again, these should fail, but it's not implemented yet
            assert!(result.is_ok())
        }
    }

    #[test]
    fn test_missing_colon_parsing() {
        let tag = parse_tag_chunk("tag").unwrap();
        assert_eq!("tag", tag.key());
        assert!(tag.value.is_empty());
    }

    #[test]
    fn test_leading_colon_parsing() {
        let _ = parse_tag_chunk(":tag").expect_err("Cannot start with a colon");
    }

    #[test]
    fn test_tailing_colon_parsing() {
        let _ = parse_tag_chunk("tag:").expect_err("Cannot end with a colon");
    }

    #[test]
    fn test_tags_parsing() {
        // See the docs for what we convey to users about tags:
        // https://docs.datadoghq.com/getting_started/tagging/

        let cases = [
            ("", vec![]),
            (",", vec![]),
            (" , ", vec![]),
            (
                "env:staging:east",
                vec![Tag::new("env", "staging:east").unwrap()],
            ),
            ("value", vec![Tag::new("value", "").unwrap()]),
            (
                "state:utah,state:idaho",
                vec![
                    Tag::new("state", "utah").unwrap(),
                    Tag::new("state", "idaho").unwrap(),
                ],
            ),
            (
                "key1:value1 key2:value2 key3:value3",
                vec![
                    Tag::new("key1", "value1").unwrap(),
                    Tag::new("key2", "value2").unwrap(),
                    Tag::new("key3", "value3").unwrap(),
                ],
            ),
            (
                // Testing consecutive separators being collapsed
                "key1:value1, key2:value2 ,key3:value3 , key4:value4",
                vec![
                    Tag::new("key1", "value1").unwrap(),
                    Tag::new("key2", "value2").unwrap(),
                    Tag::new("key3", "value3").unwrap(),
                    Tag::new("key4", "value4").unwrap(),
                ],
            ),
        ];

        for case in cases {
            let expected = case.1;
            let (actual, error_message) = parse_tags(case.0);
            assert_eq!(expected, actual);
            assert!(error_message.is_none());
        }
    }
}
