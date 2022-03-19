use super::SubscriberEmail;
use super::SubscriberName;

pub struct NewSubscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}

impl NewSubscriber {
    pub fn new(name: String, email: String) -> Result<Self, String> {
        let name = SubscriberName::try_from(name)?;
        let email = SubscriberEmail::try_from(email)?;
        Ok(Self { name, email })
    }
}
