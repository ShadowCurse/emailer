use crate::domain::SubscriberEmail;
use reqwest::Client;
use serde::Serialize;

pub struct EmailClient {
    client: Client,
    base_url: String,
    sender: SubscriberEmail,
    auth_token: String,
}

#[derive(Serialize)]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    // TODO changet base_url to reqwest::Url
    pub fn new(base_url: String, sender: SubscriberEmail, auth_token: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            sender,
            auth_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body,
            text_body,
        };
        self.client
            .post(&url)
            .header("X-Email-Server-Token", &self.auth_token)
            .json(&request_body)
            .send()
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SubscriberEmail;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn send_email_fires_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let email_client = EmailClient::new(mock_server.uri(), sender, Faker.fake());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }
}
