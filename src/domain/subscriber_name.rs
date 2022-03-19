use unicode_segmentation::UnicodeSegmentation;

pub struct SubscriberName(String);

impl TryFrom<String> for SubscriberName {
    type Error = String;
    fn try_from(value: String) -> Result<Self, String> {
        let is_empty_or_whitespase = value.trim().is_empty();
        let is_too_long = value.graphemes(true).count() > 256;

        let forbidden_chars = vec!['/', '"', '\\', '(', ')', '{', '}', '<', '>'];
        let contains_forbidden = value.chars().any(|c| forbidden_chars.contains(&c));

        if is_empty_or_whitespase || is_too_long || contains_forbidden {
            Err(format!("invalid name: {}", value))
        } else {
            Ok(Self(value))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name_valid() {
        let name = "pogdog".to_string();
        assert!(SubscriberName::try_from(name).is_ok());
    }

    #[test]
    fn long_name_256_valid() {
        let name = "a".repeat(256);
        assert!(SubscriberName::try_from(name).is_ok());
    }

    #[test]
    fn long_name_longer_then_256_invalid() {
        let name = "a".repeat(257);
        assert!(SubscriberName::try_from(name).is_err());
    }

    #[test]
    fn whitespase_only_invalid() {
        let name = " ".repeat(10);
        assert!(SubscriberName::try_from(name).is_err());
    }

    #[test]
    fn empty_invalid() {
        let name = "".to_string();
        assert!(SubscriberName::try_from(name).is_err());
    }

    #[test]
    fn contains_forbidden_chars_invalid() {
        for c in ['/', '"', '\\', '(', ')', '{', '}', '<', '>'] {
            let name = c.to_string();
            assert!(SubscriberName::try_from(name).is_err());
        }
    }
}
