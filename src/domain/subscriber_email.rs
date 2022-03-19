use validator::validate_email;

pub struct SubscriberEmail(String);

impl TryFrom<String> for SubscriberEmail {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if validate_email(&value) {
            Ok(Self(value))
        } else {
            Err(format!("invalid email: {}", value))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_email_valid() {
        let email = "pog@dog.log".to_string();
        assert!(SubscriberEmail::try_from(email).is_ok());
    }

    #[test]
    fn whitespase_only_invalid() {
        let email = " ".repeat(10);
        assert!(SubscriberEmail::try_from(email).is_err());
    }

    #[test]
    fn empty_invalid() {
        let email = "".to_string();
        assert!(SubscriberEmail::try_from(email).is_err());
    }

    #[test]
    fn missing_symbol_invalid() {
        let email = "pogdog.log".to_string();
        assert!(SubscriberEmail::try_from(email).is_err());
    }

    #[test]
    fn missing_subject_invalid() {
        let email = "@dog.log".to_string();
        assert!(SubscriberEmail::try_from(email).is_err());
    }
}
