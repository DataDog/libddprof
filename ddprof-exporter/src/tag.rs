use std::borrow::Cow;
use std::fmt::{Debug, Formatter};

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

impl Tag {
    pub fn new<IntoCow: Into<Cow<'static, str>>>(
        key: IntoCow,
        value: IntoCow,
    ) -> Result<Self, Cow<'static, str>> {
        let key = key.into();
        let value = value.into();
        match key.chars().next() {
            None => return Err("tag key was empty".into()),
            Some(char) => {
                if char == ':' {
                    return Err(format!("tag cannot start with a colon: \"{}\"", key).into());
                }
            }
        }
        if !key
            .as_ref()
            .chars()
            .filter(|char| *char != std::char::REPLACEMENT_CHARACTER && !char.is_whitespace())
            .count()
            == 0
        {
            return Err("tag contained only whitespace or UTF8 replacement characters".into());
        }

        Ok(Self { key, value })
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

#[cfg(test)]
mod tests {

    #[test]
    fn test() {}
}
