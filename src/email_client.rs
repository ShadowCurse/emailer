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
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    // TODO changet base_url to reqwest::Url
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        auth_token: String,
        timeout: std::time::Duration,
    ) -> Self {
        let client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            client,
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
            .await?
            .error_for_status()?;
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
    use wiremock::matchers::{header, header_exists, method, path};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    struct BodyMatcher;

    impl Match for BodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&request.body) {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_fires_success_200() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Faker.fake(),
            std::time::Duration::from_millis(500),
        );

        Mock::given(header_exists("X-Email-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(BodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert!(result.is_ok())
    }

    #[tokio::test]
    async fn send_email_fires_fail_500() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Faker.fake(),
            std::time::Duration::from_millis(500),
        );

        Mock::given(header_exists("X-Email-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(BodyMatcher)
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert!(result.is_err())
    }

    #[tokio::test]
    async fn send_email_fires_fail_long_responce() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender,
            Faker.fake(),
            std::time::Duration::from_millis(500),
        );

        Mock::given(header_exists("X-Email-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(BodyMatcher)
            .respond_with(ResponseTemplate::new(500).set_delay(std::time::Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert!(result.is_err())
    }
}
