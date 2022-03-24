use crate::helpers;
use wiremock::{Mock, ResponseTemplate};
use wiremock::matchers::{path, method};

#[actix_rt::test]
async fn confirmation_without_token_fail_400() {
    let test_app = helpers::spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[actix_rt::test]
async fn confirmation_link_from_subsribe_returns_200() {
    let test_app = helpers::spawn_app().await;
    let body = "name=pog%20dog&email=pogolius%40gmail.com".to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let _ = test_app.post_subsciptions(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let links = test_app.get_links(email_request);

    let response = reqwest::get(links.html)
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_rt::test]
async fn confirmation_link_confirms() {
    let test_app = helpers::spawn_app().await;
    let body = "name=pog%20dog&email=pogolius%40gmail.com".to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let _ = test_app.post_subsciptions(body).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let links = test_app.get_links(email_request);

    reqwest::get(links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("select email, name, status from subscriptions")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved sub");
    assert_eq!(saved.email, "pogolius@gmail.com");
    assert_eq!(saved.name, "pog dog");
    assert_eq!(saved.status, "confirmed");
}
