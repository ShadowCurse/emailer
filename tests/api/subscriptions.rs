use crate::helpers;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[actix_rt::test]
async fn subscribe_sends_confirmation_email() {
    let test_app = helpers::spawn_app().await;
    let body = "name=pog%20dog&email=pogolius%40gmail.com".to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let _ = test_app.post_subsciptions(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let links = test_app.get_links(email_request);
    assert_eq!(links.html, links.text);
}

#[actix_rt::test]
async fn subscribe_ret_200_if_valid_form() {
    let test_app = helpers::spawn_app().await;
    let body = "name=pog%20dog&email=pogolius%40gmail.com".to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let responce = test_app.post_subsciptions(body).await;
    assert_eq!(responce.status().as_u16(), 200);

    let saved = sqlx::query!("select email, name, status from subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "pogolius@gmail.com");
    assert_eq!(saved.name, "pog dog");
    assert_eq!(saved.status, "pending");
}

#[actix_rt::test]
async fn subscribe_ret_400_if_invalid_form() {
    let test_app = helpers::spawn_app().await;

    let invalid_forms = vec![
        ("name=pog%20dog", "missing email"),
        ("email=pogolius%40gmail.com", "missing name"),
        ("", "missing name, and email"),
        ("name=&email=pogolius%40gmail.com", "invalid name"),
        ("name=pogdog&email=some_mail_address", "invalid email"),
        ("name=pog%20dog&email=", "invalid email"),
    ];

    for (form, error) in invalid_forms {
        let responce = test_app.post_subsciptions(form.to_string()).await;

        assert_eq!(
            responce.status().as_u16(),
            400,
            "Did not fail with invalid form, expected error: {}",
            error
        );
    }
}
